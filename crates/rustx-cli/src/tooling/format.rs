//! Rustfmt owns Rust expressions; the markup pass only changes structural whitespace.
use proc_macro2::{Delimiter, LineColumn, TokenStream, TokenTree};
use std::{
    collections::HashSet,
    io::Write,
    process::{Command, Stdio},
};

/// Lines inside a multiline literal must keep their original bytes when a tag moves.
fn literal_lines(stream: TokenStream, lines: &mut HashSet<usize>) {
    for token in stream {
        match token {
            TokenTree::Group(group) => literal_lines(group.stream(), lines),
            TokenTree::Literal(literal) => {
                lines.extend(literal.span().start().line + 1..=literal.span().end().line);
            }
            _ => {}
        }
    }
}

fn reindent(text: &str, indent: &str, remove: usize) -> String {
    let mut protected = HashSet::new();
    if let Ok(tokens) = text.parse() {
        literal_lines(tokens, &mut protected);
    }
    text.lines()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 || protected.contains(&(index + 1)) {
                line.to_owned()
            } else {
                let leading = line.bytes().take_while(|b| *b == b' ').count();
                format!("{indent}{}", &line[remove.min(leading)..])
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn expression(text: &str) -> Result<String, String> {
    // A wrapper lets pinned rustfmt handle closures, blocks and nested RustX macros.
    // The visitor expression is parsed, never executed by the formatter.
    let wrapper = format!("fn __rustx_format() {{ let __rustx_value = {text}; }}");
    let formatted = source(&wrapper)?;
    let file = syn::parse_file(&formatted).map_err(|e| e.to_string())?;
    let syn::Item::Fn(function) = &file.items[0] else {
        return Err("Invalid expression formatter wrapper".into());
    };
    let syn::Stmt::Local(local) = &function.block.stmts[0] else {
        return Err("Invalid expression formatter statement".into());
    };
    use syn::spanned::Spanned;
    let span = local.init.as_ref().ok_or("Missing expression")?.expr.span();
    let text = &formatted[offset(&formatted, span.start())..offset(&formatted, span.end())];
    Ok(reindent(text, "", 4))
}

fn format_expressions(output: &mut String) -> Result<(), String> {
    let mut bodies = vec![];
    ranges(
        output.parse().map_err(|e| format!("{e}"))?,
        output,
        &mut bodies,
    );
    for (start, end) in bodies.into_iter().rev() {
        let body = &output[start..end];
        let tokens: TokenStream = body.parse().map_err(|e| format!("{e}"))?;
        let mut edits = vec![];
        for token in tokens {
            if let TokenTree::Group(group) = token {
                if group.delimiter() == Delimiter::Brace {
                    let first = offset(body, group.span_open().start());
                    let last = offset(body, group.span_close().end());
                    edits.push((first, last, expression(&body[first..last])?));
                }
            }
        }
        let mut body = body.to_owned();
        for (first, last, formatted) in edits.into_iter().rev() {
            body.replace_range(first..last, &formatted);
        }
        output.replace_range(start..end, &body);
    }
    Ok(())
}

fn tag(tokens: &[TokenTree], source: &str, indent: &str) -> Result<String, String> {
    let raw = |first: usize, last: usize| {
        &source[offset(source, tokens[first].span().start())
            ..offset(source, tokens[last].span().end())]
    };
    for pair in tokens.windows(2) {
        let gap =
            &source[offset(source, pair[0].span().end())..offset(source, pair[1].span().start())];
        if gap.contains("//") || gap.contains("/*") {
            return Ok(reindent(raw(0, tokens.len() - 1), indent, 0));
        }
    }
    // Closing tags, fragments and comments retain their exact token spelling.
    if tokens.len() < 3 || tokens[1].to_string() == "/" {
        return Ok(raw(0, tokens.len() - 1).to_owned());
    }
    let mut name_end = 1;
    while tokens
        .get(name_end + 1)
        .is_some_and(|t| t.to_string() == ":")
    {
        name_end += 3;
    }
    let name = raw(1, name_end);
    let self_closed = tokens[tokens.len() - 2].to_string() == "/";
    let end = tokens.len() - if self_closed { 2 } else { 1 };
    let mut props = vec![];
    let mut index = name_end + 1;
    while index < end {
        let begin = index;
        if index + 2 >= end || tokens[index + 1].to_string() != "=" {
            return Err("Expected a RustX property and value".into());
        }
        index += 3;
        // Unbraced values are literals in the public grammar. Keep unusual token
        // sequences untouched so the compiler can diagnose the original source.
        let property = raw(begin, index - 1);
        if property.contains("//") || property.contains("/*") {
            props.push(property.to_owned());
        } else {
            props.push(format!("{}={}", tokens[begin], raw(begin + 2, index - 1)));
        }
    }
    let close = if self_closed { " />" } else { ">" };
    let flat = format!(
        "<{name}{}{close}",
        if props.is_empty() {
            String::new()
        } else {
            format!(" {}", props.join(" "))
        }
    );
    if !flat.contains('\n') && indent.len() + flat.len() <= 100 {
        return Ok(flat);
    }
    let property_indent = format!("{indent}    ");
    let props = props
        .iter()
        .map(|p| format!("{property_indent}{}", reindent(p, &property_indent, 0)))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!("<{name}\n{props}\n{indent}{}", close.trim_start()))
}
fn offset(source: &str, point: LineColumn) -> usize {
    let prefix = source
        .split_inclusive('\n')
        .take(point.line.saturating_sub(1))
        .map(str::len)
        .sum::<usize>();
    prefix
        + source[prefix..]
            .chars()
            .take(point.column)
            .map(char::len_utf8)
            .sum::<usize>()
}
fn ranges(stream: TokenStream, source: &str, out: &mut Vec<(usize, usize)>) {
    let tokens: Vec<_> = stream.into_iter().collect();
    for (i, t) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = t {
            if i >= 2
                && tokens[i - 2].to_string() == "rustx"
                && tokens[i - 1].to_string() == "!"
                && group.delimiter() == Delimiter::Brace
            {
                out.push((
                    offset(source, group.span_open().end()),
                    offset(source, group.span_close().start()),
                ));
            } else {
                ranges(group.stream(), source, out);
            }
        }
    }
}
fn fingerprint(stream: TokenStream) -> String {
    stream
        .into_iter()
        .map(|t| match t {
            TokenTree::Group(g) => format!("G{:?}({})", g.delimiter(), fingerprint(g.stream())),
            TokenTree::Punct(p) => format!("P{};", p.as_char()),
            TokenTree::Ident(i) => format!("I{i};"),
            TokenTree::Literal(l) => format!("L{l};"),
        })
        .collect()
}
fn markup(source: &str, indent: &str) -> Result<String, String> {
    let stream: TokenStream = source
        .parse()
        .map_err(|e| format!("Invalid markup tokens: {e}"))?;
    let tokens: Vec<_> = stream.into_iter().collect();
    let mut lines = Vec::new();
    let mut depth = 0usize;
    let mut i = 0;
    let mut previous = 0;
    while i < tokens.len() {
        let start = offset(source, tokens[i].span().start());
        let gap = &source[previous..start];
        if gap.contains("//") || gap.contains("/*") {
            for line in gap.trim().lines() {
                lines.push(format!(
                    "{indent}{}{}",
                    "    ".repeat(depth + 1),
                    line.trim()
                ));
            }
        }
        let begin = i;
        let mut opening = false;
        let mut self_closed = false;
        if tokens[i].to_string() == "<" {
            let closing = tokens.get(i + 1).is_some_and(|t| t.to_string() == "/");
            opening = !closing;
            i += 1;
            while i < tokens.len() && tokens[i].to_string() != ">" {
                i += 1;
            }
            if i == tokens.len() {
                return Err("Unclosed RustX tag".into());
            }
            self_closed = i > 0 && tokens[i - 1].to_string() == "/";
            if closing {
                depth = depth.checked_sub(1).ok_or("Unexpected closing tag")?;
            }
        }
        let end = offset(source, tokens[i].span().end());
        let chunk = &source[offset(source, tokens[begin].span().start())..end];
        let line_indent = format!("{indent}{}", "    ".repeat(depth + 1));
        let content = if tokens[begin].to_string() == "<" {
            tag(&tokens[begin..=i], source, &line_indent)?
        } else {
            reindent(chunk.trim(), &line_indent, 0)
        };
        lines.push(format!("{line_indent}{content}"));
        if opening && !self_closed {
            depth += 1;
        }
        previous = end;
        i += 1;
    }
    let tail = source[previous..].trim();
    if !tail.is_empty() {
        lines.push(format!("{indent}    {tail}"));
    }
    if depth != 0 {
        return Err("Unbalanced RustX tags".into());
    }
    Ok(format!("\n{}\n", lines.join("\n")))
}
pub fn source(input: &str) -> Result<String, String> {
    syn::parse_file(input).map_err(|e| {
        format!(
            "{}:{}: {e}",
            e.span().start().line,
            e.span().start().column + 1
        )
    })?;
    let mut child = Command::new("rustup")
        .args([
            "run",
            "1.90.0",
            "rustfmt",
            "--edition",
            "2021",
            "--config",
            "newline_style=Unix",
            "--emit",
            "stdout",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .map_err(|e| e.to_string())?;
    let result = child.wait_with_output().map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).into());
    }
    let mut output = String::from_utf8(result.stdout).map_err(|e| e.to_string())?;
    format_expressions(&mut output)?;
    let parsed: TokenStream = output.parse().map_err(|e| format!("{e}"))?;
    let canonical = fingerprint(parsed.clone());
    let mut edits = vec![];
    ranges(parsed, &output, &mut edits);
    for (start, end) in edits.into_iter().rev() {
        let prefix = &output[..start];
        let indent = prefix
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect::<String>();
        let formatted = markup(&output[start..end], &indent)?;
        output.replace_range(start..end, &format!("{formatted}{indent}"));
    }
    let after: TokenStream = output.parse().map_err(|e| format!("{e}"))?;
    if canonical != fingerprint(after) {
        return Err("Formatter refused a token-changing markup edit".into());
    }
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markup_format_preserves_rust_raw_strings_comments_closures_and_unicode() {
        let text="fn app(){let _=rustx!{<Column>/* keep */<Text value={r#\"<raw> سلام\"#}/> {move || { let x = vec![1,2]; Node::text(format!(\"{x:?}\")) }} // keep line\n</Column>};}";
        let out = source(text).unwrap();
        assert!(out.contains("/* keep */"));
        assert!(out.contains("// keep line"));
        assert!(out.contains("سلام"));
        assert_eq!(source(&out).unwrap(), out);
    }
    #[test]
    fn adjacent_tags_are_not_rust_shift_operators() {
        let source_text="fn app(){let _=rustx!{<Column><Text value={if 2 >= 1 {\"yes\"} else {\"no\"}}/></Column>};}";
        let formatted = source(source_text).unwrap();
        assert!(formatted.contains("<Column>\n"));
        assert!(formatted.contains("if 2 >= 1"));
        assert_eq!(source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn windows_input_produces_consistent_idempotent_lf_output() {
        let input = "fn app() {\r\n let _ = rustx! {<Column><Text>{if true {\"سلام\"} else {\"hello\"}}</Text></Column>};\r\n}\r\n";
        let formatted = source(input).unwrap();
        assert!(!formatted.contains('\r'));
        assert_eq!(source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn multiline_props_format_nested_rust_and_preserve_literal_contents() {
        let input = r##"fn app() { let _ = rustx! { <Button /* property note */ on_click={move|_|{let x=vec![1,2];call(x)}}><Text>{r#"line one
  literal indentation
line three"#}</Text></Button> }; }"##;
        let formatted = source(input).unwrap();
        assert!(formatted.contains("/* property note */"));
        assert!(formatted.contains("let x = vec![1, 2];"));
        assert!(formatted.contains("line one\n  literal indentation\nline three"));
        assert_eq!(source(&formatted).unwrap(), formatted);
    }
}
