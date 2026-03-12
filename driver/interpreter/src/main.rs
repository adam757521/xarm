pub mod amba;
pub use amba::axi::interconnect;
pub use amba::axi::axi::Size;
pub use amba::axi::apb_bridge::Bridge;
pub use amba::apb::APB;

fn main() {
    let mut apb_bridge = Bridge::new(APB {});

    let mut inter = interconnect::Builder::new()
        .add_route(0, 0xFFFF, &mut apb_bridge)
        .build();

    inter.read(0xFFFF0, 0b000.into());
    println!("hello world");

}
