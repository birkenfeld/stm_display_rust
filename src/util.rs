// Utility macros/functions.

macro_rules! bitset {
    ($e:expr; $p:ident = true, $($tt:tt)+)    => { bitset!($e.$p().set_bit(); $($tt)+) };
    ($e:expr; $p:ident = false, $($tt:tt)+)   => { bitset!($e.$p().clear_bit(); $($tt)+) };
    ($e:expr; $p:ident = bit($v:expr), $($tt:tt)+) => { bitset!($e.$p().bit($v); $($tt)+) };
    ($e:expr; $p:ident = $v:expr, $($tt:tt)+) => { bitset!($e.$p().bits($v); $($tt)+) };
    ($e:expr; $p:ident = true)    => { $e.$p().set_bit() };
    ($e:expr; $p:ident = false)   => { $e.$p().clear_bit() };
    ($e:expr; $p:ident = bit($v:expr)) => { $e.$p().bit($v) };
    ($e:expr; $p:ident = $v:expr) => { $e.$p().bits($v) };
}

#[macro_export]
macro_rules! write {
    ($p:ident . $r:ident : $($tt:tt)+) => {
        unsafe {
            use stm::$p;
            (*$p::ptr()).$r.write(|w| bitset!(w; $($tt)+));
        }
    };
}

#[macro_export]
macro_rules! read {
    ($p:ident . $r:ident : $bit:ident) => {
        unsafe {
            use stm::$p;
            (*$p::ptr()).$r.read().$bit().bits()
        }
    };
}

#[macro_export]
macro_rules! modif {
    ($p:ident . $r:ident : $($tt:tt)+) => {
        unsafe {
            use stm::$p;
            (*$p::ptr()).$r.modify(|_, w| bitset!(w; $($tt)+));
        }
    };
}

#[macro_export]
macro_rules! wait_for {
    ($p:ident . $r:ident : $bit:ident) => {
        unsafe {
            use stm::$p;
            while (*$p::ptr()).$r.read().$bit().bit_is_clear() {}
        }
    };
    ($p:ident . $r:ident : ! $bit:ident) => {
        unsafe {
            use stm::$p;
            while (*$p::ptr()).$r.read().$bit().bit_is_set() {}
        }
    };
}

#[macro_export]
macro_rules! spi_cmd {
    ($spi:expr, $t:expr, $cs:expr, $ds:expr, $cmd:expr) => {
        $ds.set_low();
        $cs.set_low();
        spi_cmd!(@send $spi, $t, $cmd);
        $cs.set_high();
    };
    ($spi:expr, $t:expr, $cs:expr, $ds:expr, $cmd:expr, $($data:tt)+) => {
        $ds.set_low();
        $cs.set_low();
        spi_cmd!(@send $spi, $t, $cmd);
        $ds.set_high();
        spi_cmd!(@send $spi, $t, $($data)+);
        $ds.set_low();
        $cs.set_high();
    };
    (@send $spi:expr, $t:expr, $($byte:expr),+) => {
        $(
            block!($spi.send($byte)).unwrap();
            $t.delay_us(7u16);
        )+
    };
}
