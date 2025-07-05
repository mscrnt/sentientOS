.section .text._start
.global _start
.code64

_start:
    # The bootloader passes boot info address in RDI
    # Call our Rust kernel_main function
    call kernel_main
    
    # If kernel_main returns (it shouldn't), halt
    cli
.halt:
    hlt
    jmp .halt