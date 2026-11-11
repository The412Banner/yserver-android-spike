/* Minimal libxshmfence (futex variant) for the Android yserver build: the
 * shared word layout matches libxshmfence's xshmfence_futex.h, so a client's
 * libxshmfence and this server agree on the fence state. */
#include <errno.h>
#include <limits.h>
#include <stdint.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <linux/futex.h>

#define XSHMFENCE_PAGE 4096

struct xshmfence { int32_t v; };

static int futex_wake(int32_t *addr) {
    return (int)syscall(SYS_futex, addr, FUTEX_WAKE, INT_MAX, NULL, NULL, 0);
}

static int futex_wait(int32_t *addr, int32_t value) {
    return (int)syscall(SYS_futex, addr, FUTEX_WAIT, value, NULL, NULL, 0);
}

int xshmfence_trigger(struct xshmfence *f) {
    int32_t old = __sync_val_compare_and_swap(&f->v, 0, 1);
    if (old == -1) {
        old = __sync_val_compare_and_swap(&f->v, -1, 1);
        if (old == -1) futex_wake(&f->v);
    }
    return 0;
}

int xshmfence_await(struct xshmfence *f) {
    while (__sync_val_compare_and_swap(&f->v, 0, -1) != 1) {
        if (futex_wait(&f->v, -1) != 0 && errno != EWOULDBLOCK && errno != EAGAIN && errno != EINTR)
            return -1;
    }
    return 0;
}

int xshmfence_query(struct xshmfence *f) {
    return __atomic_load_n(&f->v, __ATOMIC_SEQ_CST) == 1;
}

void xshmfence_reset(struct xshmfence *f) {
    __sync_bool_compare_and_swap(&f->v, 1, 0);
}

struct xshmfence *xshmfence_map_shm(int fd) {
    void *p = mmap(NULL, XSHMFENCE_PAGE, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    return p == MAP_FAILED ? NULL : (struct xshmfence *)p;
}

void xshmfence_unmap_shm(struct xshmfence *f) {
    if (f) munmap(f, XSHMFENCE_PAGE);
}

int xshmfence_alloc_shm(void) {
    int fd = (int)syscall(__NR_memfd_create, "xshmfence", 1 /* MFD_CLOEXEC */);
    if (fd < 0) return -1;
    if (ftruncate(fd, XSHMFENCE_PAGE) < 0) { close(fd); return -1; }
    return fd;
}
