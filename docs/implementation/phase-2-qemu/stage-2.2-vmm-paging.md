# Stage 2.2: VMM + Paging

> **Goal**: Enable virtual memory with page tables.

## Tasks

1. Set up 4-level page tables (PML4)
2. Map kernel at high address (0xFFFF800000000000)
3. Identity map low memory for boot
4. Enable paging (CR0, CR3, CR4)
5. Test: allocate memory, verify isolation

## Test

Create two address spaces, verify writes don't leak.

## Next

[Stage 2.3: Interrupts + Timer](stage-2.3-interrupts-timer.md)
