#include "isr.h"
#include "stdint.h"

ISR(__defaultISR) {}

ISR(__LoadAddressMisaligned) {
    uint32_t a = 42;
    uint32_t b = 1;
    uint32_t c = a - b;
}

ISR(__ISRExceptionCrash) {
    while (1) {}
}

static void __attribute__((section (".vectortable"), naked)) vectorTable(void) {
  asm volatile(
    "j __defaultISR\n" /* UserSoftwareInterrupt */
    "j __defaultISR\n" /* SupervisorSoftwareInterrupt */
    "j __defaultISR\n" /* Reserved0 */
    "j __defaultISR\n" /* MachineSoftwareInterrupt */
    "j __defaultISR\n" /* UserTimerInterrupt */
    "j __defaultISR\n" /* SupervisorTimerInterrupt */
    "j __defaultISR\n" /* Reserved1 */
    "j __defaultISR\n" /* MachineTimerInterrupt */
    "j __defaultISR\n" /* UserExternalInterrupt */
    "j __defaultISR\n" /* SupervisorExternalInterrupt */
    "j __defaultISR\n" /* Reserved2 */
    "j __defaultISR\n" /* MachineExternalInterrupt */
    "j __ISRExceptionCrash\n" /* InstructionAddressMisaligned */
    "j __ISRExceptionCrash\n" /* InstructionAccessFault */
    "j __ISRExceptionCrash\n" /* IllegalInstruction */
    "j __defaultISR\n" /* Breakpoint */
    "j __LoadAddressMisaligned\n" /* LoadAddressMisaligned */
    "j __ISRExceptionCrash\n" /* LoadAccessFault */
    "j __ISRExceptionCrash\n" /* StoreAMOAddressMisaligned */
    "j __ISRExceptionCrash\n" /* StoreAMOAccessFault */
    "j __defaultISR\n" /* EnvironmentCallFromUMode */
    "j __defaultISR\n" /* EnvironmentCallFromSMode */
    "j __defaultISR\n" /* Reserved3 */
    "j __defaultISR\n" /* EnvironmentCallFromMMode */
    "j __defaultISR\n" /* InstructionPageFault */
    "j __ISRExceptionCrash\n" /* LoadPageFault */
    "j __defaultISR\n" /* Reserved4 */
    "j __ISRExceptionCrash\n" /* StoreAMOPageFault */
  );
}
