#![no_main]
#![no_std]
#![feature(offset_of)]

use core::fmt::Write;
use core::panic::PanicInfo;
use core::time::Duration;
use core::writeln;
use wasabi::error;
use wasabi::executor::Task;
use wasabi::executor::TimetoutFuture;
use wasabi::graphics::BitmapTextWriter;
use wasabi::hpet::global_timestamp;
use wasabi::info;
use wasabi::init::init_allocator;
use wasabi::init::init_basic_runtime;
use wasabi::init::init_display;
use wasabi::init::init_hpet;
use wasabi::init::init_paging;
use wasabi::print::hexdump;
use wasabi::println;
use wasabi::qemu::exit_qemu;
use wasabi::qemu::QemuExitCode;
use wasabi::uefi::init_vram;
use wasabi::uefi::locate_loaded_image_protocol;
use wasabi::uefi::EfiHandle;
use wasabi::uefi::EfiSystemTable;
use wasabi::warn;

use wasabi::x86::init_exceptions;

use wasabi::executor::Executor;

#[no_mangle]
fn efi_main(image_handle: EfiHandle, efi_system_table: &EfiSystemTable) {
    println!("Booting WasabiOS...\n");
    println!("image_handle:{:#018X}\n", image_handle);
    println!("efi_system_table:{:#p}\n", efi_system_table);
    let loaded_image_protocol = locate_loaded_image_protocol(image_handle, efi_system_table)
        .expect("Failed to get LoadedImageProtocol");
    println!("image_base {:018X}", loaded_image_protocol.image_base);
    println!("image_size {:018X}", loaded_image_protocol.image_size);
    info!("info!");
    warn!("warn!");
    error!("error!");
    hexdump(efi_system_table);
    let mut vram = init_vram(efi_system_table).expect("init_vram failed");
    init_display(&mut vram);
    let mut w = BitmapTextWriter::new(&mut vram);
    let acpi = efi_system_table
        .acpi_table()
        .expect("Failed to find ACPI table");
    let memory_map = init_basic_runtime(image_handle, efi_system_table);

    writeln!(w, "Hello, Non-UEFI world!").unwrap();
    init_allocator(&memory_map);

    let (_gdt, _idt) = init_exceptions();
    init_paging(&memory_map);
    init_hpet(acpi);
    let t0 = global_timestamp();
    let task1 = Task::new(async move {
        for i in 100..=103 {
            info!("{i} hpet.maincounter = {:?}", global_timestamp() - t0);
            TimetoutFuture::new(Duration::from_secs(1)).await;
        }
        Ok(())
    });
    let task2 = Task::new(async move {
        for i in 200..=203 {
            info!("{i} hpet.maincounter = {:?}", global_timestamp() - t0);
            TimetoutFuture::new(Duration::from_secs(1)).await;
        }
        Ok(())
    });

    let mut executor = Executor::new();
    executor.enqueue(task1);
    executor.enqueue(task2);
    Executor::run(executor)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info:?}");
    exit_qemu(QemuExitCode::Fail);
}
