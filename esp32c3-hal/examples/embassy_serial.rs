//! embassy serial
//!
//! This is an example of running the embassy executor and asynchronously
//! writing to and reading from uart

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use esp32c3_hal::{
    clock::ClockControl,
    embassy,
    interrupt,
    peripherals::{Interrupt, Peripherals, UART0},
    prelude::*,
    Uart,
};
use esp_backtrace as _;
use esp_hal_common::uart::{config::AtCmdConfig, UartRx, UartTx};
use heapless::Vec;

// rx_fifo_full_threshold
const READ_BUF_SIZE: usize = 64;
// EOT (CTRL-D)
const AT_CMD: u8 = 0x04;

#[embassy_executor::task]
async fn writer(mut tx: UartTx<'static, UART0>) {
    esp_println::println!("writing...");
    embedded_io_async::Write::write(
        &mut tx,
        b"Hello async serial. Enter something ended with EOT (CTRL-D).\r\n",
    )
    .await
    .unwrap();
    embedded_io_async::Write::flush(&mut tx).await.unwrap();
}

#[embassy_executor::task]
async fn reader(mut rx: UartRx<'static, UART0>) {
    esp_println::println!("reading...");
    // max message size to receive
    // leave some extra space for AT-CMD characters
    const MAX_BUFFER_SIZE: usize = 10 * READ_BUF_SIZE + 16;

    let mut rbuf: Vec<u8, MAX_BUFFER_SIZE> = Vec::new();
    let mut offset = 0;
    while let Ok(len) = embedded_io_async::Read::read(&mut rx, &mut rbuf[offset..]).await {
        offset += len;
        if offset == 0 {
            rbuf.truncate(0);
            break;
        }
        // if set_at_cmd is used than stop reading
        if len < READ_BUF_SIZE {
            rbuf.truncate(offset);
            break;
        }
    }
}

#[main]
async fn main(spawner: Spawner) {
    esp_println::println!("Init!");
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    #[cfg(feature = "embassy-time-systick")]
    embassy::init(
        &clocks,
        esp32c3_hal::systimer::SystemTimer::new(peripherals.SYSTIMER),
    );

    #[cfg(feature = "embassy-time-timg0")]
    embassy::init(
        &clocks,
        esp32c3_hal::timer::TimerGroup::new(peripherals.TIMG0, &clocks).timer0,
    );

    let mut uart0 = Uart::new(peripherals.UART0, &clocks);
    uart0.set_at_cmd(AtCmdConfig::new(None, None, None, AT_CMD, None));
    uart0
        .set_rx_fifo_full_threshold(READ_BUF_SIZE as u16)
        .unwrap();
    let (tx, rx) = uart0.split();

    interrupt::enable(Interrupt::UART0, interrupt::Priority::Priority1).unwrap();

    spawner.spawn(reader(rx)).ok();
    spawner.spawn(writer(tx)).ok();
}
