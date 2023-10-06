use std::collections::{HashSet, HashMap};

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{self, Data, DataEnum, DataStruct, DeriveInput, Fields, Type, parse_macro_input};

fn impl_derive_config(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let fields = 
    // Check if the input is a struct
    if let Data::Struct(DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = &ast.data
    {
        fields
    } else {
        // Throw an error if the input is not a struct
        panic!("#[derive(Config)] can only be used with structs");
    };

    struct MyField {
        name: String,
        ty: String,
    }
    

    let mut print_info = vec![];

    

    let mut fields_and_names: Vec<MyField> = vec![];

    for field in &fields.named {
        if let Some(name) = &field.ident {

            if let Type::Path(type_path) = &field.ty {
                let field_type = type_path.path.segments.last().unwrap().ident.to_string();
                
                match field_type.as_str() {
                    "f64" | "bool" | "String" | "i64" | "i32" => {
                        // Type is valid, continue processing

                        fields_and_names.push(MyField{name: name.to_string(), ty: field_type});
                    }
                    _ => {
                        panic!("#[derive(Config)] can only be used with fields of type f64, bool, String, i64, or i32");
                    }
                }
            }

            // Print name;
            for attr in &field.attrs {


            }
        }
    }
    

    for item in fields_and_names {
        let name = item.name;
        let ty = item.ty;
        print_info.push(quote!{
            println!("\t{}: {}", #name, #ty);
        });
    }
    


        // impl Display for #name {
        //     fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        //         if let Ok(s) = serde_json::to_string_pretty(self) {
        //             f.write_str(&s)
        //         } else {
        //             Err(std::fmt::Error::default())
        //         }
        //     }
        // }


    //let fields_and_names = vec!["test".to_owned(), "asdf".to_owned(), "qwerty".to_owned()];

    let gen = quote! {
        impl #name {
            pub fn print_fields() {
                #(#print_info)*
            }
        }
    };
    gen.into()
}

struct ParamAttr {
    name: syn::Ident,
    test_value: syn::Ident,
}

impl syn::parse::Parse for ParamAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let test_value = input.parse()?;
        //let _: syn::Token![=] = input.parse()?;
        
        Ok(ParamAttr { name, test_value})
    }
}

// #[proc_macro_attribute]
// pub fn param2(args: TokenStream, input: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(input as DeriveInput);


//     panic!("Getting into the proc macro");
//     TokenStream::from(quote::quote!(#input))

// }


#[proc_macro_derive(Config, attributes(param))]
pub fn config(input: TokenStream) -> TokenStream {
    // Parse the string representation
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let param = ast.attrs.iter()
        .find(|attr| attr.path.is_ident("param"))
        .map(|attr| attr.parse_args::<ParamAttr>())
        .transpose();




    // // Build the impl
    impl_derive_config(&ast)
}

#[cfg(test)]
mod tests {
    //
}
