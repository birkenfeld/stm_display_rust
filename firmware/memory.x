MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  FLASH : ORIGIN = 0x08000000, LENGTH = 2048K
  RAM   : ORIGIN = 0x10000000, LENGTH = 64K      /* CPU coupled */
  SRAM1 : ORIGIN = 0x20000000, LENGTH = 112K
  SRAM2 : ORIGIN = 0x2001C000, LENGTH = 16K
  SRAM3 : ORIGIN = 0x20020000, LENGTH = 64K
}

/* This is where the call stack will be allocated. */
/* The stack is of the full descending type. */
/* You may want to use this variable to locate the call stack and static
   variables in different memory regions. Below is shown the default value */
/* _stack_start = ORIGIN(RAM) + LENGTH(RAM); */

/* You can use this symbol to customize the location of the .text section */
/* If omitted the .text section will be placed right after the .vector_table
   section */
/* This is required only on microcontrollers that store some configuration right
   after the vector table */
/* _stext = ORIGIN(FLASH) + 0x400; */

/* Place dedicated (frame)buffers into SRAMs. */

SECTIONS
{
  .sram1bss (NOLOAD) : ALIGN(4)
  {
    *(.sram1bss);
    . = ALIGN(4);
  } > SRAM1

  .sram2bss (NOLOAD) : ALIGN(4)
  {
    *(.sram2bss);
    . = ALIGN(4);
  } > SRAM2

  .sram3bss (NOLOAD) : ALIGN(4)
  {
    *(.sram3bss);
    . = ALIGN(4);
  } > SRAM3
}
INSERT AFTER .bss;

/* Ensure FW_IDENT is put into the binary, at the end. */

EXTERN(FW_IDENT);

SECTIONS
{
  .fw_ident : ALIGN(4)
  {
    *(.fw_ident);
    . = ALIGN(4);
  } > FLASH
}
INSERT AFTER .rodata;
