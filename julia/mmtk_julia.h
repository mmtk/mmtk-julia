#include "mmtk.h"
#include "gc.h"

extern Julia_Upcalls mmtk_upcalls;

int set_gc_running_state(jl_ptls_t ptls);
void mmtk_jl_gc_run_all_finalizers(void);
void mmtk_jl_run_pending_finalizers(void* tls);
