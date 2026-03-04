// TODO: Imrpove this whole system lol
#[macro_export]
macro_rules! call_boot {
    ($st:expr, $fn_name:ident $(, $arg:expr )* $(,)? ) => {{
        let svc_ptr = unsafe { ((*$st.boot_services)).$fn_name };
        Into::<$crate::errors::Result<()>>::into(svc_ptr( $( $arg ),* ))
    }};
}

#[macro_export]
macro_rules! call_runtime {
    ($st:expr, $fn_name:ident $(, $arg:expr )* $(,)? ) => {{
        let svc_ptr = unsafe { ((*$st.runtime_services)).$fn_name };
        Into::<$crate::errors::Result<()>>::into(svc_ptr( $( $arg ),* ))
    }};
}

#[macro_export]
macro_rules! efi_main {
    ($func:path) => {
        #[unsafe(no_mangle)]
        pub extern "win64" fn efi_main(
            image_handle: $crate::types::Handle,
            system_table: *mut $crate::types::SystemTable,
        ) -> $crate::types::Status {
            let res: $crate::errors::Result<()> =
                $func(image_handle, unsafe { system_table.as_mut().unwrap() });
            $crate::types::Status(match res {
                Ok(_) => 0,
                Err(err) => err.code.0,
            })
        }
    };
}
