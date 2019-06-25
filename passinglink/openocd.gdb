target remote :3333
set print asm-demangle on
monitor arm semihosting enable

set remotetimeout unlimited

# detect unhandled exceptions, hard faults and panics
# break DefaultHandler
# break HardFault
# break rust_begin_unwind

load
