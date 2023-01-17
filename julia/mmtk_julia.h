#include "mmtk.h"
#include "gc.h"

extern Julia_Upcalls mmtk_upcalls;

void calculate_roots(void* ptls);

void run_finalizer_function(void *o, void *ff, bool is_ptr);

uintptr_t get_jl_last_err(void);

void set_jl_last_err(uintptr_t e);

size_t get_lo_size(bigval_t obj);

int8_t set_gc_initial_state(void* ptls);

void set_gc_final_state(int8_t old_state);

int set_gc_running_state(jl_ptls_t ptls);

void set_gc_old_state(int8_t old_state);

void mark_object_as_scanned(void* obj);

int8_t object_has_been_scanned(void* obj);

void mmtk_jl_gc_run_all_finalizers(void);

void mmtk_jl_run_finalizers(void* tls);

void mmtk_jl_run_pending_finalizers(void* tls);

JL_DLLEXPORT void scan_julia_obj(void* obj, closure_pointer closure, ProcessEdgeFn process_edge, ProcessOffsetEdgeFn process_offset_edge);