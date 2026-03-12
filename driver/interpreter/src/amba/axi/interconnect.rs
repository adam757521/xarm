use super::axi::{Slave, Size, Data, Beat};

// TODO: awfully slow simple implementation.
// This is similar to decoder router.

// Should we use Cow for the slave?
struct Route<'a> {
    range_start: u32,
    range_end: u32,
    slave: &'a mut dyn Slave
}

// NOTE: no emulation of write/read queues.
pub struct Interconnect<'a> {
    routes: Box<[Route<'a>]>,
}

impl<'a> Interconnect<'a> {
    fn route(&'a mut self, address: u32) -> Option<&'a mut dyn Slave> {
        let mut cands = self.routes.iter_mut().filter(|r| r.range_start <= address && address <= r.range_end);

        let first = cands.next();

        match (first, cands.next()) {
            (None, _) => None,
            (Some(route), None) => Some(route.slave),
            (Some(_), Some(_)) => panic!("Multiple routes match address 0x{:08X}", address),
        }
    }

    // Interconnect Interface
    pub fn read(&'a mut self, addr: u32, size: Size) -> Data {
        let beat = Beat::new(addr, None, size);
        self.route(addr).unwrap().process_beat(&beat).unwrap()
    }

    pub fn write(&'a mut self, addr: u32, size: Size, data: Vec<u8>) {
        let beat = Beat::new(addr, Some(data), size);
        self.route(addr).unwrap().process_beat(&beat);
    }
}

#[derive(Default)]
pub struct Builder<'a> {
    routes: Vec<Route<'a>>
}

impl<'a> Builder<'a> {
    pub fn new() -> Self {
        Self {
            routes: vec![],
        }
    }

    pub fn add_route(mut self, start: u32, end: u32, slave: &'a mut dyn Slave) -> Self {
        self.routes.push(Route {
            range_start: start,
            range_end: end,
            slave
        });
        self
    }

    // TODO: Better to handle overlap here than at runtime.
    pub fn build(self) -> Interconnect<'a> {
        Interconnect {
            routes: Box::from(self.routes)
        }
    }
}
