.section .text
.global guest_boot
guest_boot:
    li a7, 1        # Extension ID: 1 (legacy console)
    li a6, 0        # Function ID: 0 (putchar)

    li a0, 'A'      # Parameter: 'A'
    ecall           # Call SBI (hypervisor)
    li a0, 'B'      # Parameter: 'B'
    ecall           # Call SBI (hypervisor)
    li a0, 'C'      # Parameter: 'C'
    ecall           # Call SBI (hypervisor)

halt:
    j halt          # Infinite loop
