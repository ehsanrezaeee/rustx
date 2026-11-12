use super::*;
use quote::ToTokens;
use syn::visit::Visit;
struct Props(Vec<String>);
impl<'ast> Visit<'ast> for Props {
    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        if node
            .expr
            .to_token_stream()
            .to_string()
            .contains("id . to_string")
        {
            fn names(p: &syn::Pat, out: &mut Vec<String>) {
                match p {
                    syn::Pat::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) => out.push(s.value()),
                    syn::Pat::Or(p) => {
                        for item in &p.cases {
                            names(item, out)
                        }
                    }
                    _ => {}
                }
            }
            for arm in &node.arms {
                names(&arm.pat, &mut self.0);
            }
        }
        syn::visit::visit_expr_match(self, node);
    }
}
pub fn catalog() -> Value {
    let widgets =
        syn::parse_file(include_str!(concat!(env!("OUT_DIR"), "/core-widgets.rs"))).unwrap();
    let mut tags = vec!["Div".to_owned(), "Input".to_owned()];
    for item in widgets.items {
        if let syn::Item::Enum(e) = item {
            if e.ident == "Widget" {
                tags.extend(e.variants.into_iter().map(|v| v.ident.to_string()));
            }
        }
    }
    tags.sort();
    tags.dedup();
    let mut props = Props(vec!["key".into()]);
    props.visit_file(
        &syn::parse_file(include_str!(concat!(env!("OUT_DIR"), "/macros.rs"))).unwrap(),
    );
    props.0.sort();
    props.0.dedup();
    json!({"tags":tags,"props":props.0})
}
pub fn project(root: &Path) -> Result<Value, String> {
    let mut paths = vec![];
    files(root, &mut paths)?;
    let mut symbols = vec![];
    for path in paths {
        if !matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("rs" | "rustx")
        ) {
            continue;
        }
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let Ok(file) = syn::parse_file(&text) else {
            continue;
        };
        struct Functions<'a> {
            path: &'a Path,
            symbols: &'a mut Vec<Value>,
        }
        impl<'ast> Visit<'ast> for Functions<'_> {
            fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
                let component = f.attrs.iter().any(|a| a.path().is_ident("component"));
                let props = f
                    .sig
                    .inputs
                    .iter()
                    .filter_map(|a| {
                        if let syn::FnArg::Typed(a) = a {
                            if let syn::Pat::Ident(p) = &*a.pat {
                                if p.ident != "cx" {
                                    return Some(p.ident.to_string());
                                }
                            }
                            None
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let at = f.sig.ident.span().start();
                self.symbols.push(json!({"name":f.sig.ident.to_string(),"component":component,"props":props,"file":self.path,"line":at.line.saturating_sub(1),"character":at.column}));
                syn::visit::visit_item_fn(self, f);
            }
        }
        Functions {
            path: &path,
            symbols: &mut symbols,
        }
        .visit_file(&file);
    }
    Ok(json!({"catalog":catalog(),"symbols":symbols}))
}
