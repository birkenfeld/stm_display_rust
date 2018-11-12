//! Millisecond upcounting timer.

use stm;
use hal;

pub struct Millis {
    tim: stm::TIM2
}

impl Millis {
    pub fn new(tim2: stm::TIM2, clocks: hal::rcc::Clocks) -> Self {
        modif!(RCC.apb1enr: tim2en = true);
        modif!(RCC.apb1rstr: tim2rst = true);
        modif!(RCC.apb1rstr: tim2rst = false);
        write!(TIM2.psc: psc = (clocks.pclk1().0 / 1000) as u16 - 1);
        write!(TIM2.egr: ug = true);
        write!(TIM2.cr1: udis = true, cen = true);
        Millis { tim: tim2 }
    }

    pub fn get(&self) -> u32 {
        read!(TIM2.cnt: cnt)
    }
}
