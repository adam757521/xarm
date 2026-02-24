use proc_macro::TokenStream;

use quote::quote;
use syn::{FnArg, ItemFn, ReturnType, Type, parse_macro_input};

#[proc_macro_attribute]
pub fn entry_point(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let sig = &input.sig;

    match &sig.output {
        ReturnType::Type(_, ty) if matches!(**ty, Type::Never(_)) => {}
        _ => {
            return syn::Error::new_spanned(&sig.ident, "expected function to return `!`")
                .to_compile_error()
                .into();
        }
    }

    if sig.inputs.len() != 1 {
        return syn::Error::new_spanned(
            &sig.inputs,
            "expected function to have exactly one argument",
        )
        .to_compile_error()
        .into();
    }

    let arg = &sig.inputs[0];
    if let FnArg::Typed(pat_type) = arg {
        let type_str = quote!(#pat_type.ty).to_string();
        // TODO: unsafe lol
        if !type_str.contains("HandoffPayload") {
            return syn::Error::new_spanned(
                &pat_type.ty,
                "argument must be of type 'boot::HandoffPayload'",
            )
            .to_compile_error()
            .into();
        }
    } else {
        return syn::Error::new_spanned(arg, "Invalid argument type")
            .to_compile_error()
            .into();
    }

    let ident = &sig.ident;
    let expanded = quote! {
        #input

        use ::boot::_export::uefi as __uefi;

        __uefi::efi_main!(__uefi_boot_entry);
        fn __uefi_boot_entry(
            handle: __uefi::Handle,
            st: &mut __uefi::SystemTable
        ) -> __uefi::Result<()> {
            let trampoline_payload = unsafe { ::boot::core::init(handle, st)? };
            unsafe {
                ::boot::handoff::handoff(trampoline_payload, #ident);
            }
        }
    };

    TokenStream::from(expanded)
}
