// We need a ZST type of thing, but it makes sense
// for the look type to hold the word and just take it as reference or actually copy is better
// just needs to be thing
//
// Either do add.rd(raw)
// or it handles the thing behind the thing and do add.rd()
//

use crate::ir;

use quote::quote;
use proc_macro2::TokenStream;

pub fn emit(instructions: &[&ir::Instruction]) -> TokenStream {
    let members = instructions.iter().map(|i| {
        let name = quote::format_ident!("{}", i.name.to_string());
        quote! {
            #name
        }
    });

    quote! {
        #[repr(u16)]
        #[derive(Debug)]
        pub enum InstructionView {
            #(#members),*
        }
    }
}
