use proc_macro2::*;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Attribute, Data, DataStruct, DeriveInput, Expr, ExprLit,
    Fields, Lit, LitInt, Type, TypeArray, TypePath, Field, punctuated::Punctuated, ExprAssign, Token, parse::Parser, token::Comma, ExprPath, Path,
};

#[proc_macro_attribute]
pub fn derive_desc(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let s = parse_macro_input!(item as DeriveInput);

    dbg!(s.attrs.clone());

    let mut ok = false;
    let ref repr_attr: Attribute = parse_quote!(#[repr(C)]);
    for attr in &s.attrs {
        if attr == repr_attr {
            ok = true;
            break;
        }
    }
    assert!(ok, "需要是 #[repr(C)] 才行");

    // let step_mode = parse_macro_input!(attr as Ident);
    let name = s.ident.clone();
    let Data::Struct(DataStruct{fields, ..}) = s.data.clone() else {panic!("只能用在 Struct 上")};

    let len = fields.len();
    let (offsets,i, tys) = match fields {
        Fields::Named(fields) => {
            let l = fields.named.len();
            let i = 0..l;
            let (idents, tys): (Vec<_>, Vec<_>) = fields
                .named
                .into_iter()
                .map(|f| (f.ident.unwrap(), get_wgpu_vertex_format(f.ty)))
                .unzip();
            let offsets : Vec<_>= idents.into_iter().map(|ident|{
                quote!(memoffset::offset_of!(#name, #ident))
            }).collect();
            (offsets, i, tys)
        }
        Fields::Unnamed(fields) => {
            let l = fields.unnamed.len();
            let i = 0..l;
            let tys:Vec<_> = fields.unnamed.into_iter().map(|f|{
                get_wgpu_vertex_format(f.ty)
            }).collect();
            let offsets : Vec<_>= i.clone().into_iter().map(|i|{
                quote!(memoffset::offset_of_tuple!(#name, #i))
            }).collect();
            (offsets, i, tys)
        }
        Fields::Unit => panic!("不能是空的 Struct"),
    };

    let attrs = quote! {
        [#(
            wgpu::VertexAttribute {
                offset: #offsets as wgpu::BufferAddress,
                shader_location: #i as u32,
                format: #tys,
            },
        )*]
    };

    let ret = quote! {
        #s

        impl #name {
            fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
                static ATTRS : once_cell::sync::OnceCell<[VertexAttribute;#len]> = once_cell::sync::OnceCell::new();
                let attrs = ATTRS.get_or_init(||{
                    #attrs
                });
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<#name>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: attrs,
                }
            }
        }
    };

    println!("{}", &ret);

    ret.into()
}

fn get_wgpu_vertex_format(ty: Type) -> TokenStream {
    let f = match ty {
        Type::Array(TypeArray { elem, len, .. }) => match *elem {
            Type::Path(ty) => {
                let mut per = get_wgpu_vertex_format_perfix(ty);
                if let Expr::Lit(ExprLit {
                    lit: Lit::Int(lit), ..
                }) = len
                {
                    let len = lit.base10_parse::<u32>().unwrap();
                    if len > 1 {
                        per += &("x".to_string() + &lit.to_string());
                    }
                } else {
                    panic!("不支持这个类型 3")
                };

                per
            }
            _ => panic!("不支持这个类型 2"),
        },
        Type::Path(ty) => get_wgpu_vertex_format_perfix(ty),
        _ => panic!("不支持这个类型 1"),
    };
    let ident = Ident::new(&f, Span::call_site());
    quote!(wgpu::VertexFormat::#ident)
}

fn get_wgpu_vertex_format_perfix(ty: TypePath) -> String {
    let t = quote!(#ty).to_string();
    let c = t.bytes().nth(0).unwrap() as char;
    let a: String = match c {
        'i' => "Sint",
        'u' => "Uint",
        'f' => "Float",
        _ => {
            dbg!(ty,t,c);
            panic!("不支持这个类型 aaaa");
        },
    }
    .into();
    let ident = a + &t[1..];
    return ident;
}
