//! JSX-shaped syntax lowered into ordinary typed Rust expressions.
use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::{format_ident, quote};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, Expr, FnArg, Ident, ItemFn, LitStr, Pat, Result, Token,
};

struct Markup(Tokens);
fn children(input: ParseStream, close: bool) -> Result<Vec<Tokens>> {
    let mut out = vec![];
    while !input.is_empty() {
        if close && input.peek(Token![<]) && input.peek2(Token![/]) {
            break;
        }
        out.push(input.parse::<Markup>()?.0);
    }
    Ok(out)
}
impl Parse for Markup {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(syn::token::Brace) {
            let body;
            braced!(body in input);
            let expr: Expr = body.parse()?;
            return Ok(Self(if matches!(expr, Expr::Closure(_)) {
                quote!(::rustx::dynamic(#expr))
            } else {
                quote!(::rustx::into_view(#expr))
            }));
        }
        if input.peek(LitStr) {
            let s: LitStr = input.parse()?;
            return Ok(Self(quote!(::rustx::Node::text(#s))));
        }
        input.parse::<Token![<]>()?;
        if input.peek(Token![>]) {
            input.parse::<Token![>]>()?;
            let kids = children(input, true)?;
            input.parse::<Token![<]>()?;
            input.parse::<Token![/]>()?;
            input.parse::<Token![>]>()?;
            return Ok(Self(quote!(::rustx::Node::fragment(vec![#(#kids),*]))));
        }
        let tag: Ident = input.parse()?;
        let name = tag.to_string();
        let builtin = matches!(
            name.as_str(),
            "Div"
                | "Input"
                | "Box"
                | "Row"
                | "Column"
                | "Grid"
                | "Stack"
                | "Spacer"
                | "Divider"
                | "ScrollView"
                | "Text"
                | "Heading"
                | "Paragraph"
                | "Image"
                | "Icon"
                | "Button"
                | "IconButton"
                | "Link"
                | "Tabs"
                | "Menu"
                | "MenuItem"
                | "Breadcrumbs"
                | "Form"
                | "Label"
                | "TextInput"
                | "PasswordInput"
                | "TextArea"
                | "Checkbox"
                | "RadioGroup"
                | "Switch"
                | "Select"
                | "Combobox"
                | "Slider"
                | "Dialog"
                | "Popover"
                | "Tooltip"
                | "Alert"
                | "Toast"
                | "Progress"
                | "Spinner"
                | "List"
                | "VirtualList"
                | "Table"
                | "TableRow"
                | "TableCell"
                | "Canvas"
                | "DrawingSurface"
        );
        if name.chars().next().is_some_and(char::is_lowercase) {
            return Err(syn::Error::new(
                tag.span(),
                "RustX tags start uppercase: <Div>, <Text>, <Button>, or a #[component]",
            ));
        }
        let mut attrs = vec![];
        let mut key = None;
        let mut seen = std::collections::HashSet::new();
        while !input.peek(Token![>]) && !input.peek(Token![/]) {
            let ident: Ident = input.parse()?;
            let canonical = match ident.to_string().as_str() {
                "onClick" => "on_click".to_owned(),
                "onInput" => "on_input".to_owned(),
                _ => ident.to_string(),
            };
            if !seen.insert(canonical) {
                return Err(syn::Error::new(ident.span(), "duplicate property"));
            }
            input.parse::<Token![=]>()?;
            let value: Expr = if input.peek(syn::token::Brace) {
                let body;
                braced!(body in input);
                body.parse()?
            } else {
                let literal: syn::Lit = input.parse()?;
                syn::parse_quote!(#literal)
            };
            if ident == "key" {
                key = Some(value);
            } else {
                attrs.push((ident, value));
            }
        }
        let kids = if input.peek(Token![/]) {
            input.parse::<Token![/]>()?;
            input.parse::<Token![>]>()?;
            vec![]
        } else {
            input.parse::<Token![>]>()?;
            let kids = children(input, true)?;
            input.parse::<Token![<]>()?;
            input.parse::<Token![/]>()?;
            let end: Ident = input.parse()?;
            if end != tag {
                return Err(syn::Error::new(
                    end.span(),
                    format!("expected closing </{tag}>"),
                ));
            }
            input.parse::<Token![>]>()?;
            kids
        };
        let mut result = if builtin {
            let mut setters = vec![];
            for (id, value) in attrs {
                let method = match id.to_string().as_str() {
                    "style" => quote!(style),
                    "id" => quote!(id),
                    "onClick" | "on_click" => quote!(on_click),
                    "onInput" | "on_input" | "on_change" => quote!(on_input),
                    "value" => quote!(value),
                    "value_text" => quote!(value_text),
                    "value_now" => quote!(value_now),
                    "label" => quote!(label),
                    "node_ref" => quote!(node_ref),
                    "role" => quote!(role),
                    "editing" => quote!(editing),
                    "control" => quote!(control),
                    "disabled" => quote!(disabled),
                    "read_only" => quote!(read_only),
                    "selectable" => quote!(selectable),
                    "checked" => quote!(checked),
                    "selected" => quote!(selected),
                    "expanded" => quote!(expanded),
                    "modal" => quote!(modal),
                    "overlay" => quote!(overlay),
                    "anchor" => quote!(anchor),
                    "hidden" => quote!(hidden),
                    "min" => quote!(min),
                    "max" => quote!(max),
                    "step" => quote!(step),
                    "options" => quote!(options),
                    "described_by" => quote!(described_by),
                    "labelled_by" => quote!(labelled_by),
                    "controls" => quote!(controls),
                    "tab_index" => quote!(tab_index),
                    "href" => quote!(href),
                    "input_type" => quote!(input_type),
                    "lang" => quote!(lang),
                    "required" => quote!(required),
                    "invalid" => quote!(invalid),
                    "live" => quote!(live),
                    "level" => quote!(level),
                    "sort" => quote!(sort),
                    "on_key_down" => quote!(on_key_down),
                    "on_key_up" => quote!(on_key_up),
                    "on_pointer_down" => quote!(on_pointer_down),
                    "on_pointer_move" => quote!(on_pointer_move),
                    "on_pointer_up" => quote!(on_pointer_up),
                    "on_pointer_cancel" => quote!(on_pointer_cancel),
                    "on_pointer_enter" => quote!(on_pointer_enter),
                    "on_pointer_leave" => quote!(on_pointer_leave),
                    "on_before_input" => quote!(on_before_input),
                    "on_composition_start" => quote!(on_composition_start),
                    "on_composition_update" => quote!(on_composition_update),
                    "on_composition_end" => quote!(on_composition_end),
                    "on_copy" => quote!(on_copy),
                    "on_cut" => quote!(on_cut),
                    "on_paste" => quote!(on_paste),
                    "on_file_drop" => quote!(on_file_drop),
                    "on_resize" => quote!(on_resize),
                    "on_resource_error" => quote!(on_resource_error),
                    "on_submit" => quote!(on_submit),
                    "on_reset" => quote!(on_reset),
                    "on_blur" => quote!(on_blur),
                    "on_wheel" => quote!(on_wheel),
                    "on_focus" => quote!(on_focus),
                    "on_scroll" => quote!(on_scroll),
                    "drawing" if name == "Canvas" => quote!(drawing),
                    "camera" if name == "Canvas" => quote!(camera),
                    "on_canvas" if name == "Canvas" => quote!(on_canvas),
                    "src" if name == "Image" => quote!(src),
                    _ => {
                        return Err(syn::Error::new(
                            id.span(),
                            "unknown property; supported: style, id, key, onClick",
                        ))
                    }
                };
                setters.push(quote!(.#method(#value)));
            }
            let constructor = match name.as_str() {
                "Div" => quote!(::rustx::Node::new(::rustx::Kind::Div)),
                "Input" => quote!(::rustx::Node::new(::rustx::Kind::Input)),
                // Existing primitive tags retain their established default style.
                "Text" | "Button" | "Image" | "Canvas" => {
                    quote!(::rustx::Node::new(::rustx::Kind::#tag))
                }
                _ => quote!(::rustx::Node::widget(::rustx::Widget::#tag)),
            };
            quote!(#constructor #(#setters)* .children(vec![#(#kids),*]))
        } else {
            let props = format_ident!("{}Props", tag);
            let setters = attrs.into_iter().map(|(id, v)| quote!(.#id(#v)));
            let childprop = if kids.is_empty() {
                quote!()
            } else {
                quote!(.children(vec![#(#kids),*]))
            };
            quote!(#tag(#props::builder() #(#setters)* #childprop .build()))
        };
        if let Some(key) = key {
            result = quote!({let __rustx_key=(#key).to_string();(#result).key(__rustx_key)});
        }
        Ok(Self(result))
    }
}
struct Document(Vec<Tokens>);
impl Parse for Document {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(children(input, false)?))
    }
}
#[proc_macro]
pub fn rustx(input: TokenStream) -> TokenStream {
    let Document(nodes) = parse_macro_input!(input as Document);
    if nodes.len() == 1 {
        nodes[0].clone().into()
    } else {
        quote!(::rustx::Node::fragment(vec![#(#nodes),*])).into()
    }
}

#[proc_macro_attribute]
pub fn component(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as ItemFn);
    match expand(f) {
        Ok(v) => v.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
fn expand(f: ItemFn) -> Result<Tokens> {
    if f.sig.asyncness.is_some() || f.sig.generics.lifetimes().next().is_some() {
        return Err(syn::Error::new_spanned(
            &f.sig,
            "Components must be synchronous with owned props; lifetime generics are unsupported",
        ));
    }
    let name = &f.sig.ident;
    let props = format_ident!("{}Props", name);
    let bodyname = format_ident!("__rustx_body_{}", name);
    let visibility = &f.vis;
    let body = &f.block;
    let output = &f.sig.output;
    let mut generics = f.sig.generics.clone();
    for param in generics.type_params_mut() {
        param.bounds.push(syn::parse_quote!(Clone));
        param.bounds.push(syn::parse_quote!('static));
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let mut fields = vec![];
    let mut args = vec![];
    let mut params = vec![];
    for arg in &f.sig.inputs {
        let FnArg::Typed(p) = arg else {
            return Err(syn::Error::new_spanned(arg, "component cannot have self"));
        };
        let Pat::Ident(id) = &*p.pat else {
            return Err(syn::Error::new_spanned(
                p,
                "component props must be named arguments",
            ));
        };
        let ident = &id.ident;
        let ty = &p.ty;
        if ident == "cx" {
            params.push(quote!(cx: ::rustx::Scope));
            args.push(quote!(__cx.clone()));
            continue;
        }
        let mut defaults = quote!();
        for attr in &p.attrs {
            if attr.path().is_ident("prop") {
                let tokens = attr.meta.require_list()?.tokens.clone();
                defaults = quote!(#[builder(#tokens)]);
            }
        }
        fields.push(quote!(#defaults pub #ident:#ty));
        args.push(quote!(__props.#ident.clone()));
        params.push(quote!(#ident:#ty));
    }
    Ok(quote! {
           #[derive(Clone,::rustx::__private::typed_builder::TypedBuilder)]
    #[builder(crate_module_path=::rustx::__private::typed_builder)]
    #visibility struct #props #impl_generics #where_clause {#(#fields),*}
           #[allow(non_snake_case)] fn #bodyname #impl_generics (#(#params),*) #output #where_clause #body
           #[allow(non_snake_case)] #visibility fn #name #impl_generics (__props:#props #type_generics)->::rustx::View #where_clause {
               ::rustx::Node::component(::std::any::type_name::<#props #type_generics>(),move |__cx|#bodyname(#(#args),*))
           }
       })
}
