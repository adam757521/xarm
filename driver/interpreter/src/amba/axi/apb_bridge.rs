use super::super::apb::APB;
use super::axi::{Slave, Beat, Data};

pub struct Bridge {
    apb: APB
}

impl Bridge {
    pub fn new(apb: APB) -> Self {
        Self {
            apb
        }
    }
}

impl Slave for Bridge {
    fn process_beat(&mut self, beat: &Beat) -> Option<Data> {
        println!("{beat:?}");
        None
    }
}
