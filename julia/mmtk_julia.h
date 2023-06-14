#include "mmtk.h"
#include "gc.h"

extern Julia_Upcalls mmtk_upcalls;

int get_jl_last_err(void);

void set_jl_last_err(int e);

void set_gc_final_state(int8_t old_state);

int set_gc_running_state(jl_ptls_t ptls);

void set_gc_old_state(int8_t old_state);

void mmtk_jl_gc_run_all_finalizers(void);

void mmtk_jl_run_pending_finalizers(void* tls);
