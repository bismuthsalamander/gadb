#include <stdio.h>
#include <sys/signal.h>
#include <unistd.h>

int main() {
    unsigned long long a = 0x1badd00d2badf00d;
    unsigned long long *ptr = &a;
    write(STDOUT_FILENO, &ptr, sizeof(void*));
    fflush(stdout);
    raise(SIGTRAP);

    char b[12] = { 0 };
    char *b_ptr = &b;
    write(STDOUT_FILENO, &b_ptr, sizeof(void*));
    fflush(stdout);
    raise(SIGTRAP);

    printf("%s", b);
}