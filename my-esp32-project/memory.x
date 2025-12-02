MEMORY
{
  /*
    https://github.com/espressif/esp-idf/blob/9f1ffc4dd5a5e155d69f3d7d14e3b52ce7aee09a/components/esp_system/ld/esp32c3/memory.ld.in
  */
  vectors_seg ( RX )     : ORIGIN = 0x40380000, LENGTH = 0x400
  
  /* SRAM1; we leave 32k for the bootloader and ROM stacks etc.
     Instruction and data SRAM are integrated togeter */
  ICACHE ( RX )          : ORIGIN = 0x40380000 + 0x400, LENGTH = 400K - 0x400
  IRAM ( RX )            : ORIGIN = 0x40380000 + 0x400, LENGTH = 400K - 0x400
  DCACHE ( RW )          : ORIGIN = 0x3FC80000, LENGTH = 0x50000
  DRAM ( RW )            : ORIGIN = 0x3FC80000, LENGTH = 0x50000

  /* external flash
     The 0x20 offset is a convenience for the app binary image generation.
     Flash cache has 64KB pages. The .bin file which is flashed to the chip
     has a 0x18 byte file header, and each segment has a 0x08 byte segment
     header. Setting this offset makes it simple to meet the flash cache MMU's
     constraint that (paddr % 64KB == vaddr % 64KB).)
  */
  IROM ( RX )            : ORIGIN = 0x42000000 + 0x20, LENGTH = 0x400000 - 0x20
  DROM ( R )             : ORIGIN = 0x3C000000 + 0x20, LENGTH = 0x400000 - 0x20

  /* RTC fast memory (executable). Persists over deep sleep. Only for core 0 (PRO_CPU) */
  RTC_FAST ( RWX )       : ORIGIN = 0x50000000, LENGTH = 0x2000

  /* RTC slow memory (data accessible). Persists over deep sleep. */
  RTC_SLOW ( RW )        : ORIGIN = 0x50000000 + 0x2000, LENGTH = 0x2000
}
