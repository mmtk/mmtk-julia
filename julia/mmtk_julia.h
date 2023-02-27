#include "mmtk.h"
#include "gc.h"

extern Julia_Upcalls mmtk_upcalls;

void calculate_roots(jl_ptls_t ptls);

void run_finalizer_function(jl_value_t *o, jl_value_t *ff, bool is_ptr);

int get_jl_last_err(void);

void set_jl_last_err(int e);

size_t get_lo_size(bigval_t obj);

int8_t set_gc_initial_state(jl_ptls_t ptls);

void set_gc_final_state(int8_t old_state);

int set_gc_running_state(jl_ptls_t ptls);

void set_gc_old_state(int8_t old_state);

void mark_object_as_scanned(jl_value_t* obj);

int8_t object_has_been_scanned(jl_value_t* obj);

void mmtk_jl_gc_run_all_finalizers(void);

void mmtk_jl_run_finalizers(jl_ptls_t tls);

void mmtk_jl_run_pending_finalizers(void* tls);

JL_DLLEXPORT void scan_julia_obj(jl_value_t* obj, closure_pointer closure, ProcessEdgeFn process_edge, ProcessOffsetEdgeFn process_offset_edge);
