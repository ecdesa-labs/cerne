//! The attributes of [Cerne](https://docs.rs/cerne): `#[entity]` and `#[aggregate]`.
//!
//! Use them from `cerne::domain`, where they are documented: this crate is only how Rust builds them.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Error, Fields, GenericArgument, Meta, PathArguments, Type, parse_macro_input,
};

/// Writes the identity and the constructor of a 🟨 entity.
#[proc_macro_attribute]
pub fn entity(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let is_aggregate = false;

    expand(arguments, item, is_aggregate)
}

/// Writes the identity and the constructor of a 🟨 aggregate, and marks it as one.
#[proc_macro_attribute]
pub fn aggregate(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let is_aggregate = true;

    expand(arguments, item, is_aggregate)
}

fn expand(arguments: TokenStream, item: TokenStream, is_aggregate: bool) -> TokenStream {
    let entity = parse_macro_input!(item as DeriveInput);
    let attribute = if is_aggregate { "aggregate" } else { "entity" };

    if !arguments.is_empty() {
        let no_arguments = Error::new_spanned(
            Tokens::from(arguments),
            format!("#[{attribute}] takes no arguments"),
        );

        return no_arguments.to_compile_error().into();
    }

    entity_with_its_identity_and_constructor(entity, attribute, is_aggregate)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// The struct as written (without `#[skip_constructor]`), its `<Name>Constructor`, `impl Entity` and, for an
/// aggregate, `impl Aggregate`.
fn entity_with_its_identity_and_constructor(
    mut entity: DeriveInput,
    attribute: &str,
    is_aggregate: bool,
) -> syn::Result<Tokens> {
    let name = entity.ident.clone();
    let visibility = entity.vis.clone();
    let constructor = format_ident!("{name}Constructor");

    if !entity.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &entity.generics,
            format!("#[{attribute}] does not take generics"),
        ));
    }

    let Data::Struct(data) = &mut entity.data else {
        return Err(Error::new_spanned(
            &entity.ident,
            format!("#[{attribute}] goes on a struct with named fields"),
        ));
    };

    let Fields::Named(fields) = &mut data.fields else {
        return Err(Error::new_spanned(
            &entity.ident,
            format!("#[{attribute}] goes on a struct with named fields"),
        ));
    };

    // --- The fields: the id, the constructor's and the skipped ---------------

    let mut id_type = None;
    let mut constructor_fields = vec![];
    let mut constructor_field_names = vec![];
    let mut skipped_fields = vec![];

    for field in fields.named.iter_mut() {
        let field_name = field.ident.clone().expect("named fields have a name");
        let skip_constructor = take_skip_constructor(&mut field.attrs)?;

        if field_name == "id" {
            id_type = Some(option_inner_type(&field.ty).cloned().ok_or_else(|| {
                Error::new_spanned(
                    &field.ty,
                    "the id is an Option: None until the repository saves the entity, like id: Option<OrderId>",
                )
            })?);
        } else if skip_constructor {
            skipped_fields.push(field_name);
        } else {
            let docs: Vec<_> = field
                .attrs
                .iter()
                .filter(|attribute| attribute.path().is_ident("doc"))
                .collect();
            let field_visibility = &field.vis;
            let field_type = &field.ty;

            constructor_fields.push(quote! {
                #(#docs)*
                #field_visibility #field_name: #field_type
            });
            constructor_field_names.push(field_name);
        }
    }

    let Some(id_type) = id_type else {
        return Err(Error::new_spanned(
            &entity.ident,
            format!(
                "#[{attribute}] needs a field id: Option<{name}Id>, None until the repository saves it"
            ),
        ));
    };

    // --- What the attribute writes -------------------------------------------

    let constructor_doc = format!(
        "What a new `{name}` is made of. There is no id: the repository decides it on the first `save`. \
         A field marked `#[skip_constructor]` starts at its `Default`."
    );

    let aggregate = is_aggregate.then(|| {
        quote! {
            #[automatically_derived]
            impl ::cerne::domain::Aggregate for #name {}
        }
    });

    Ok(quote! {
        #entity

        #[doc = #constructor_doc]
        #visibility struct #constructor {
            #(#constructor_fields,)*
        }

        #aggregate

        #[automatically_derived]
        impl ::cerne::domain::Entity for #name {
            type Id = #id_type;
            type Constructor = #constructor;

            fn id(&self) -> ::core::option::Option<&#id_type> {
                self.id.as_ref()
            }

            fn with_id(mut self, id: #id_type) -> Self {
                self.id = ::core::option::Option::Some(id);

                self
            }

            fn new(constructor: #constructor) -> ::cerne::domain::EnforcementResult<Self> {
                let #constructor { #(#constructor_field_names,)* } = constructor;

                ::cerne::domain::Validate::validate(Self {
                    id: ::core::option::Option::None,
                    #(#constructor_field_names,)*
                    #(#skipped_fields: ::core::default::Default::default(),)*
                })
            }
        }
    })
}

/// Removes `#[skip_constructor]` from a field and says whether it was there.
fn take_skip_constructor(attributes: &mut Vec<syn::Attribute>) -> syn::Result<bool> {
    let mut skip_constructor = false;

    for attribute in attributes.iter() {
        if attribute.path().is_ident("skip_constructor") {
            if !matches!(attribute.meta, Meta::Path(_)) {
                return Err(Error::new_spanned(
                    attribute,
                    "#[skip_constructor] takes no arguments: the field starts at its Default",
                ));
            }

            skip_constructor = true;
        }
    }

    attributes.retain(|attribute| !attribute.path().is_ident("skip_constructor"));

    Ok(skip_constructor)
}

/// `OrderId` in `Option<OrderId>`.
fn option_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    let PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return None;
    };

    match arguments.args.first()? {
        GenericArgument::Type(inner) if last.ident == "Option" => Some(inner),
        _ => None,
    }
}
