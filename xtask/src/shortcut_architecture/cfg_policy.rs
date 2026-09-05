//! Semantic evaluation of whether a syntax item can exist outside test builds.

pub(super) fn has_test_only_configuration(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        if !attribute.path().is_ident("cfg") {
            return false;
        }
        attribute
            .parse_args::<syn::Meta>()
            .is_ok_and(|meta| !cfg_possibility(&meta).can_be_true)
    })
}

#[derive(Clone, Copy)]
struct CfgPossibility {
    can_be_true: bool,
    can_be_false: bool,
}

fn cfg_possibility(meta: &syn::Meta) -> CfgPossibility {
    match meta {
        syn::Meta::Path(path) if path.is_ident("test") => CfgPossibility {
            can_be_true: false,
            can_be_false: true,
        },
        syn::Meta::Path(_) | syn::Meta::NameValue(_) => CfgPossibility {
            can_be_true: true,
            can_be_false: true,
        },
        syn::Meta::List(list) => {
            let nested = list
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                )
                .map_or_else(
                    |_| {
                        vec![CfgPossibility {
                            can_be_true: true,
                            can_be_false: true,
                        }]
                    },
                    |items| {
                        items
                            .into_iter()
                            .map(|item| cfg_possibility(&item))
                            .collect()
                    },
                );
            if list.path.is_ident("all") {
                CfgPossibility {
                    can_be_true: nested.iter().all(|item| item.can_be_true),
                    can_be_false: nested.iter().any(|item| item.can_be_false),
                }
            } else if list.path.is_ident("any") {
                CfgPossibility {
                    can_be_true: nested.iter().any(|item| item.can_be_true),
                    can_be_false: nested.iter().all(|item| item.can_be_false),
                }
            } else if list.path.is_ident("not") && nested.len() == 1 {
                CfgPossibility {
                    can_be_true: nested[0].can_be_false,
                    can_be_false: nested[0].can_be_true,
                }
            } else {
                CfgPossibility {
                    can_be_true: true,
                    can_be_false: true,
                }
            }
        }
    }
}
