// Defines identical string literals in two different C files and checks that they end up pointing
// to the same memory.

#include "exit.h"

extern const char s1h[];
extern const char s2h[];
extern const char s1w[];
extern const char s2w[];

const char* get_loc1(void);

void _start(void) {
    if (s1h != s2h) {
        exit_syscall(101);
    }
    if (s1w != s2w) {
        exit_syscall(102);
    }
    if (get_loc1()[0] != 'L') {
        exit_syscall(103);
    }
    exit_syscall(42);
}
