use proc_macro2::TokenStream;

pub trait CodeEmitter {
    fn emit() -> TokenStream;
}
