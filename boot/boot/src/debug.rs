use uefi::SystemTable;
use uefi::MemoryType;
use uefi;

pub fn print_u64(st: *mut SystemTable, mut n: u64) {
    let mut buf = [0u16; 20]; // max 20 digits for u64
    let mut i = buf.len();

    if n == 0 {
        buf[i - 1] = '0' as u16;
        i -= 1;
    } else {
        while n > 0 {
            i -= 1;
            buf[i] = ('0' as u16) + (n % 10) as u16;
            n /= 10;
        }
    }

    unsafe {
        let s = &buf[i..];
        let con_out = (*st).con_out.as_mut().unwrap();
        (con_out.output_string)(con_out, s.as_ptr());
    }
}

pub fn print_hex(st: *mut SystemTable, mut number: u64) {
    let mut buf = [0u16; 19];
    buf[18] = 0xa as u16;
    const LUT: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
        'a', 'b', 'c', 'd', 'e', 'f'
    ];

    let mut i = buf.len()-1;
    while number > 0 {
        buf[i - 1] = LUT[(number & 0xf) as usize] as u16;
        number >>= 4;
        i -= 1;
    }

    if i == buf.len()-1 {
        buf[i - 1] = '0' as u16;
        i -= 1;
    }

    buf[i-1] = 'x' as u16;
    buf[i-2] = '0' as u16;

    unsafe {
        let s = &buf[(i-2)..];
        let con_out = (*st).con_out.as_mut().unwrap();
        (con_out.output_string)(con_out, s.as_ptr());
    }
}

pub fn print_arbit_message(st: &mut SystemTable, message: &str) -> uefi::Result<()> {
    //This shouldnt allocate, or be a singleton, but it will not exist
    let mut buffer: *mut uefi::VOID = core::ptr::null_mut();
    uefi::call_boot!(
        st,
        allocate_pool,
        MemoryType::EfiLoaderData,
        (message.len() + 1) * 2,
        &mut buffer,
    )?;

    for (i, c) in message.encode_utf16().enumerate() {
        unsafe {
            // nice..
            *(buffer as *mut u16).add(i) = c;
        }
    }

    unsafe {
        *(buffer as *mut u16).add(message.len()) = 0;
    }

    unsafe {
        let con_out = (*st).con_out.as_mut().unwrap();
        (con_out.output_string)(con_out, buffer as *const u16);

        uefi::call_boot!(st, free_pool, buffer)?;
    };

    Ok(())
}
