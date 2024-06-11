// install bindgen with cargo install bindgen-cli
// run:
// BINDGEN_EXTRA_CLANG_ARGS="-I mmtk/api" ~/.cargo/bin/bindgen julia/mmtk_julia_types.h --opaque-type MMTkMutatorContext -o mmtk/src/julia_types.rs
#include <setjmp.h>	
#include <stdint.h>
#include <pthread.h>
#include "mmtkMutator.h"

typedef __SIZE_TYPE__ size_t;
typedef int sig_atomic_t;

struct mmtk__jl_taggedvalue_bits {
    uintptr_t gc:2;
    uintptr_t in_image:1;
};

typedef struct mmtk__jl_value_t mmtk_jl_value_t;
typedef struct mmtk__jl_taggedvalue_t mmtk_jl_taggedvalue_t;

struct mmtk__jl_taggedvalue_t {
    union {
        uintptr_t header;
        mmtk_jl_taggedvalue_t *next;
        mmtk_jl_value_t *type; // 16-byte aligned
        struct mmtk__jl_taggedvalue_bits bits;
    };
    // jl_value_t value;
};

typedef struct {
    /*
      how - allocation style
      0 = data is inlined, or a foreign pointer we don't manage
      1 = julia-allocated buffer that needs to be marked
      2 = malloc-allocated pointer this array object manages
      3 = has a pointer to the object that owns the data
    */
    unsigned short int how:2;
    unsigned short int ndims:9;
    unsigned short int pooled:1;
    unsigned short int ptrarray:1; // representation is pointer array
    unsigned short int hasptr:1; // representation has embedded pointers
    unsigned short int isshared:1; // data is shared by multiple Arrays
    unsigned short int isaligned:1; // data allocated with memalign
} mmtk_jl_array_flags_t;

typedef struct {
    uint32_t size;
    uint32_t nfields;
    uint32_t npointers; // number of pointers embedded inside
    int32_t first_ptr; // index of the first pointer (or -1)
    uint16_t alignment; // strictest alignment over all fields
    uint16_t haspadding : 1; // has internal undefined bytes
    uint16_t fielddesc_type : 2; // 0 -> 8, 1 -> 16, 2 -> 32, 3 -> foreign type
    // union {
    //     jl_fielddesc8_t field8[nfields];
    //     jl_fielddesc16_t field16[nfields];
    //     jl_fielddesc32_t field32[nfields];
    // };
    // union { // offsets relative to data start in words
    //     uint8_t ptr8[npointers];
    //     uint16_t ptr16[npointers];
    //     uint32_t ptr32[npointers];
    // };
} mmtk_jl_datatype_layout_t;

typedef struct {
    void *name;
    struct mmtk__jl_module_t *module;
    void *names;  // field names
    const uint32_t *atomicfields; // if any fields are atomic, we record them here
    const uint32_t *constfields; // if any fields are const, we record them here
    // `wrapper` is either the only instantiation of the type (if no parameters)
    // or a UnionAll accepting parameters to make an instantiation.
    void *wrapper;
    void *Typeofwrapper;
    void *cache;        // sorted array
    void *linearcache;  // unsorted array
    void *mt;
    void *partial;     // incomplete instantiations of this type
    intptr_t hash;
    int32_t n_uninitialized;
    // type properties
    uint8_t abstract:1;
    uint8_t mutabl:1;
    uint8_t mayinlinealloc:1;
    uint8_t _reserved:5;
    uint8_t max_methods;
} mmtk_jl_typename_t;

typedef struct {
    size_t length;
    // pointer size aligned
    // jl_value_t *data[];
} mmtk_jl_svec_t;

typedef struct mmtk__jl_datatype_t {
    mmtk_jl_typename_t *name;
    struct mmtk__jl_datatype_t *super;
    mmtk_jl_svec_t *parameters;
    mmtk_jl_svec_t *types;
    mmtk_jl_value_t *instance;  // for singletons
    const mmtk_jl_datatype_layout_t *layout;
    // memoized properties
    uint32_t hash;
    uint8_t hasfreetypevars:1; // majority part of isconcrete computation
    uint8_t isconcretetype:1; // whether this type can have instances
    uint8_t isdispatchtuple:1; // aka isleaftupletype
    uint8_t isbitstype:1; // relevant query for C-api and type-parameters
    uint8_t zeroinit:1; // if one or more fields requires zero-initialization
    uint8_t has_concrete_subtype:1; // If clear, no value will have this datatype
    uint8_t cached_by_hash:1; // stored in hash-based set cache (instead of linear cache)
    uint8_t isprimitivetype:1; // whether this is declared with 'primitive type' keyword (sized, no fields, and immutable)
} mmtk_jl_datatype_t;

typedef struct {
    void *data;
    size_t length;
    mmtk_jl_array_flags_t flags;
    uint16_t elsize;  // element size including alignment (dim 1 memory stride)
    uint32_t offset;  // for 1-d only. does not need to get big.
    size_t nrows;
    union {
        // 1d
        size_t maxsize;
        // Nd
        size_t ncols;
    };
    // other dim sizes go here for ndims > 2

    // followed by alignment padding and inline data, or owner pointer
} mmtk_jl_array_t;

typedef struct mmtk__jl_sym_t {
    _Atomic(void *) left;
    _Atomic(void *) right;
    uintptr_t hash;    // precomputed hash value
    // JL_ATTRIBUTE_ALIGN_PTRSIZE(char name[]);
} mmtk_jl_sym_t;

typedef struct {
    // not first-class
    void *name;
    void* value;
    void* globalref;  // cached GlobalRef for this binding
    struct _jl_module_t* owner;  // for individual imported bindings -- TODO: make _Atomic
    void* ty;  // binding type
    uint8_t constp:1;
    uint8_t exportp:1;
    uint8_t imported:1;
    uint8_t deprecated:2; // 0=not deprecated, 1=renamed, 2=moved to another package
} mmtk_jl_binding_t;

#define HT_N_INLINE 32

typedef struct {
    size_t size;
    void **table;
    void *_space[HT_N_INLINE];
} mmtk_htable_t;

#define AL_N_INLINE 29

typedef struct {
    size_t len;
    size_t max;
    void **items;
    void *_space[AL_N_INLINE];
} mmtk_arraylist_t;

#define SMALL_AL_N_INLINE 6

typedef struct {
    uint32_t len;
    uint32_t max;
    void **items;
    void *_space[SMALL_AL_N_INLINE];
} mmtk_small_arraylist_t;

typedef struct {
    uint64_t hi;
    uint64_t lo;
} mmtk_jl_uuid_t;

typedef struct {
    _Atomic(void*) owner;
    uint32_t count;
} mmtk_jl_mutex_t;

typedef struct mmtk__jl_module_t {
    void *name;
    struct mmtk__jl_module_t *parent;
    // hidden fields:
    mmtk_htable_t bindings;
    mmtk_arraylist_t usings;  // modules with all bindings potentially imported
    mmtk_jl_uuid_t build_id;
    mmtk_jl_uuid_t uuid;
    size_t primary_world;
    _Atomic(uint32_t) counter;
    int32_t nospecialize;  // global bit flags: initialization for new methods
    int8_t optlevel;
    int8_t compile;
    int8_t infer;
    uint8_t istopmod;
    int8_t max_methods;
    mmtk_jl_mutex_t lock;
} mmtk_jl_module_t;

// Exception stack: a stack of pairs of (exception,raw_backtrace).
// The stack may be traversed and accessed with the functions below.
struct mmtk__jl_excstack_t { // typedef in julia.h
    size_t top;
    size_t reserved_size;
    // Pack all stack entries into a growable buffer to amortize allocation
    // across repeated exception handling.
    // Layout: [bt_data1... bt_size1 exc1  bt_data2... bt_size2 exc2  ..]
    // jl_bt_element_t data[]; // Access with jl_excstack_raw
};

typedef struct mmtk__jl_bt_element_t {
    union {
        uintptr_t   uintptr; // Metadata or native instruction ptr
        void* jlvalue; // Pointer to GC-managed value
    };
} mmtk_jl_bt_element_t;

typedef struct mmtk__jl_excstack_t mmtk_jl_excstack_t;

#ifdef	__USE_POSIX
/* Use the same type for `jmp_buf' and `sigjmp_buf'.
   The `__mask_was_saved' flag determines whether
   or not `longjmp' will restore the signal mask.  */
// typedef struct __jmp_buf_tag sigjmp_buf[1];

/* Store the calling environment in ENV, also saving the
   signal mask if SAVEMASK is nonzero.  Return 0.  */
# define sigsetjmp(env, savemask)	__sigsetjmp (env, savemask)

/* Jump to the environment saved in ENV, making the
   sigsetjmp call there return VAL, or 1 if VAL is 0.
   Restore the signal mask if that sigsetjmp call saved it.
   This is just an alias `longjmp'.  */
extern void siglongjmp (sigjmp_buf __env, int __val)
     __THROWNL __attribute__ ((__noreturn__));
#endif /* Use POSIX.  */

#  define mmtk_jl_jmp_buf sigjmp_buf
#  if defined(_CPU_ARM_) || defined(_CPU_PPC_) || defined(_CPU_WASM_)
#    define MAX_ALIGN 8
#  elif defined(_CPU_AARCH64_)
// int128 is 16 bytes aligned on aarch64
#    define MAX_ALIGN 16
#  elif defined(_P64)
// Generically we assume MAX_ALIGN is sizeof(void*)
#    define MAX_ALIGN 8
#  else
#    define MAX_ALIGN 4
#  endif

typedef struct {
    mmtk_jl_jmp_buf uc_mcontext;
} mmtk_jl_stack_context_t;

typedef mmtk_jl_stack_context_t mmtk__jl_ucontext_t;

typedef struct {
    union {
        mmtk__jl_ucontext_t ctx;
        mmtk_jl_stack_context_t copy_ctx;
    };
#if defined(_COMPILER_TSAN_ENABLED_)
    void *tsan_state;
#endif
#if defined(_COMPILER_ASAN_ENABLED_)
    void *asan_fake_stack;
#endif
} mmtk_jl_ucontext_t;

typedef struct mmtk__jl_gcframe_t mmtk_jl_gcframe_t;

struct mmtk__jl_gcframe_t {
    size_t nroots;
    struct mmtk__jl_gcframe_t *prev;
    // actual roots go here
};

typedef struct {
    mmtk_jl_taggedvalue_t *freelist;   // root of list of free objects
    mmtk_jl_taggedvalue_t *newpages;   // root of list of chunks of free objects
    uint16_t osize;      // size of objects in this pool
} mmtk_jl_gc_pool_t;

typedef struct {
    _Atomic(int64_t) allocd;
    _Atomic(int64_t) pool_live_bytes;
    _Atomic(int64_t) freed;
    _Atomic(uint64_t) malloc;
    _Atomic(uint64_t) realloc;
    _Atomic(uint64_t) poolalloc;
    _Atomic(uint64_t) bigalloc;
    _Atomic(uint64_t) freecall;
} mmtk_jl_thread_gc_num_t;

typedef struct {
    // variable for tracking weak references
    mmtk_small_arraylist_t weak_refs;
    // live tasks started on this thread
    // that are holding onto a stack from the pool
    mmtk_small_arraylist_t live_tasks;

    // variables for tracking malloc'd arrays
    struct _mallocarray_t *mallocarrays;
    struct _mallocarray_t *mafreelist;

    // variables for tracking big objects
    struct _bigval_t *big_objects;

    // variables for tracking "remembered set"
    mmtk_arraylist_t rem_bindings;
    mmtk_arraylist_t _remset[2]; // contains jl_value_t*
    // lower bound of the number of pointers inside remembered values
    int remset_nptr;
    mmtk_arraylist_t *remset;
    mmtk_arraylist_t *last_remset;

    // variables for allocating objects from pools
#define JL_GC_N_MAX_POOLS 51
    mmtk_jl_gc_pool_t norm_pools[JL_GC_N_MAX_POOLS];

#define JL_N_STACK_POOLS 16
    mmtk_small_arraylist_t free_stacks[JL_N_STACK_POOLS];
} mmtk_jl_thread_heap_t;

// handle to reference an OS thread
typedef pthread_t mmtk_jl_thread_t;

typedef struct {
    _Atomic(int64_t) top;
    _Atomic(int64_t) bottom;
    _Atomic(void *) array;
} mmtk_ws_queue_t;

typedef struct {
    mmtk_ws_queue_t chunk_queue;
    mmtk_ws_queue_t ptr_queue;
    mmtk_arraylist_t reclaim_set;
} mmtk_jl_gc_markqueue_t;

typedef struct {
    // thread local increment of `perm_scanned_bytes`
    size_t perm_scanned_bytes;
    // thread local increment of `scanned_bytes`
    size_t scanned_bytes;
    // Number of queued big objects (<= 1024)
    size_t nbig_obj;
    // Array of queued big objects to be moved between the young list
    // and the old list.
    // A set low bit means that the object should be moved from the old list
    // to the young list (`mark_reset_age`).
    // Objects can only be put into this list when the mark bit is flipped to
    // `1` (atomically). Combining with the sync after marking,
    // this makes sure that a single objects can only appear once in
    // the lists (the mark bit cannot be flipped to `0` without sweeping)
    void *big_obj[1024];
} mmtk_jl_gc_mark_cache_t;

typedef struct {
    void* bottom;
} mmtk_jl_gc_page_stack_t;

typedef struct mmtk__jl_tls_states_t {
    int16_t tid;
    int8_t threadpoolid;
    uint64_t rngseed;
    volatile size_t *safepoint;
    _Atomic(int8_t) sleep_check_state; // read/write from foreign threads
    // Whether it is safe to execute GC at the same time.
#define JL_GC_STATE_WAITING 1
    // gc_state = 1 means the thread is doing GC or is waiting for the GC to
    //              finish.
#define JL_GC_STATE_SAFE 2
    // gc_state = 2 means the thread is running unmanaged code that can be
    //              execute at the same time with the GC.
    _Atomic(int8_t) gc_state; // read from foreign threads
    // execution of certain certain impure
    // statements is prohibited from certain
    // callbacks (such as generated functions)
    // as it may make compilation undecidable
    int8_t in_pure_callback;
    int8_t in_finalizer;
    int8_t disable_gc;
    // Counter to disable finalizer **on the current thread**
    int finalizers_inhibited;
    mmtk_jl_thread_heap_t heap; // this is very large, and the offset is baked into codegen
    mmtk_jl_thread_gc_num_t gc_num;
    volatile sig_atomic_t defer_signal;
    _Atomic(struct mmtk__jl_task_t*) current_task;
    struct mmtk__jl_task_t *next_task;
    struct mmtk__jl_task_t *previous_task;
    struct mmtk__jl_task_t *root_task;
    void *timing_stack;
    void *stackbase;
    size_t stacksize;
    union {
        mmtk__jl_ucontext_t base_ctx; // base context of stack
        // This hack is needed to support always_copy_stacks:
        mmtk_jl_stack_context_t copy_stack_ctx;
    };
    // Temp storage for exception thrown in signal handler. Not rooted.
    mmtk_jl_value_t *sig_exception;
    // Temporary backtrace buffer. Scanned for gc roots when bt_size > 0.
    struct mmtk__jl_bt_element_t *bt_data; // JL_MAX_BT_SIZE + 1 elements long
    size_t bt_size;    // Size for backtrace in transit in bt_data
    // Temporary backtrace buffer used only for allocations profiler.
    struct mmtk__jl_bt_element_t *profiling_bt_buffer;
    // Atomically set by the sender, reset by the handler.
    volatile _Atomic(sig_atomic_t) signal_request; // TODO: no actual reason for this to be _Atomic
    // Allow the sigint to be raised asynchronously
    // this is limited to the few places we do synchronous IO
    // we can make this more general (similar to defer_signal) if necessary
    volatile sig_atomic_t io_wait;
    void *signal_stack;
    mmtk_jl_thread_t system_id;
    mmtk_arraylist_t finalizers;
    mmtk_jl_gc_page_stack_t page_metadata_allocd;
    mmtk_jl_gc_page_stack_t page_metadata_buffered;
    mmtk_jl_gc_markqueue_t mark_queue;
    mmtk_jl_gc_mark_cache_t gc_cache;
    mmtk_arraylist_t sweep_objs;
    _Atomic(int64_t) gc_sweeps_requested;
    // Saved exception for previous *external* API call or NULL if cleared.
    // Access via jl_exception_occurred().
    struct _jl_value_t *previous_exception;

    // currently-held locks, to be released when an exception is thrown
    mmtk_small_arraylist_t locks;

    MMTkMutatorContext mmtk_mutator;
    size_t malloc_sz_since_last_poll;

    // JULIA_DEBUG_SLEEPWAKE(
    //     uint64_t uv_run_enter;
    //     uint64_t uv_run_leave;
    //     uint64_t sleep_enter;
    //     uint64_t sleep_leave;
    // )

    // some hidden state (usually just because we don't have the type's size declaration)
} mmtk_jl_tls_states_t;

typedef struct mmtk__jl_task_t {
    void *next; // invasive linked list for scheduler
    void *queue; // invasive linked list for scheduler
    void *tls;
    void *donenotify;
    void *result;
    void *logstate;
    void *start;
    uint64_t rngState[4];
    _Atomic(uint8_t) _state;
    uint8_t sticky; // record whether this Task can be migrated to a new thread
    _Atomic(uint8_t) _isexception; // set if `result` is an exception to throw or that we exited with
    // multiqueue priority
    uint16_t priority;

// hidden state:
    // id of owning thread - does not need to be defined until the task runs
    _Atomic(int16_t) tid;
    // threadpool id
    int8_t threadpoolid;
    // saved gc stack top for context switches
    mmtk_jl_gcframe_t *gcstack;
    // stack of objects (not slots) that need to be transitively pinned
    mmtk_jl_gcframe_t *tpin_gcstack;
    size_t world_age;
    // quick lookup for current ptls
    void* ptls; // == jl_all_tls_states[tid]
    // saved exception stack
    mmtk_jl_excstack_t *excstack;
    // current exception handler
    void *eh;
    // saved thread state
    mmtk_jl_ucontext_t ctx;
    void *stkbuf; // malloc'd memory (either copybuf or stack)
    size_t bufsz; // actual sizeof stkbuf
    uint64_t inference_start_time; // time when inference started
    uint16_t reentrant_inference; // How many times we've reentered inference
    uint16_t reentrant_timing; // How many times we've reentered timing
    unsigned int copy_stack:31; // sizeof stack for copybuf
    unsigned int started:1;
} mmtk_jl_task_t;

typedef struct {
    mmtk_jl_value_t *value;
} mmtk_jl_weakref_t;

// the following mirrors `struct EffectsOverride` in `base/compiler/effects.jl`
typedef union mmtk___jl_purity_overrides_t {
    struct {
        uint8_t ipo_consistent          : 1;
        uint8_t ipo_effect_free         : 1;
        uint8_t ipo_nothrow             : 1;
        uint8_t ipo_terminates_globally : 1;
        // Weaker form of `terminates` that asserts
        // that any control flow syntactically in the method
        // is guaranteed to terminate, but does not make
        // assertions about any called functions.
        uint8_t ipo_terminates_locally  : 1;
        uint8_t ipo_notaskstate         : 1;
        uint8_t ipo_inaccessiblememonly : 1;
    } overrides;
    uint8_t bits;
} mmtk__jl_purity_overrides_t;

// This type describes a single method definition, and stores data
// shared by the specializations of a function.
typedef struct mmtk__jl_method_t {
    void *name;  // for error reporting
    struct mmtk__jl_module_t *module;
    void *file;
    int32_t line;
    size_t primary_world;
    size_t deleted_world;

    // method's type signature. redundant with TypeMapEntry->specTypes
    void *sig;

    // table of all jl_method_instance_t specializations we have
    _Atomic(void*) specializations; // allocated as [hashable, ..., NULL, linear, ....], or a single item
    _Atomic(void*) speckeyset; // index lookup by hash into specializations

    void *slot_syms; // compacted list of slot names (String)
    void *external_mt; // reference to the method table this method is part of, null if part of the internal table
    void *source;  // original code template (jl_code_info_t, but may be compressed), null for builtins
    _Atomic(void*) unspecialized;  // unspecialized executable method instance, or null
    void *generator;  // executable code-generating function if available
    void *roots;  // pointers in generated code (shared to reduce memory), or null
    // Identify roots by module-of-origin. We only track the module for roots added during incremental compilation.
    // May be NULL if no external roots have been added, otherwise it's a Vector{UInt64}
    void *root_blocks;   // RLE (build_id.lo, offset) pairs (even/odd indexing)
    int32_t nroots_sysimg;     // # of roots stored in the system image
    void *ccallable; // svec(rettype, sig) if a ccallable entry point is requested for this

    // cache of specializations of this method for invoke(), i.e.
    // cases where this method was called even though it was not necessarily
    // the most specific for the argument types.
    _Atomic(void*) invokes;

    // A function that compares two specializations of this method, returning
    // `true` if the first signature is to be considered "smaller" than the
    // second for purposes of recursion analysis. Set to NULL to use
    // the default recursion relation.
    void *recursion_relation;

    uint32_t nargs;
    uint32_t called;        // bit flags: whether each of the first 8 arguments is called
    uint32_t nospecialize;  // bit flags: which arguments should not be specialized
    uint32_t nkw;           // # of leading arguments that are actually keyword arguments
                            // of another method.
    // various boolean properties
    uint8_t isva;
    uint8_t pure;
    uint8_t is_for_opaque_closure;
    // uint8 settings
    uint8_t constprop;      // 0x00 = use heuristic; 0x01 = aggressive; 0x02 = none

    // Override the conclusions of inter-procedural effect analysis,
    // forcing the conclusion to always true.
    mmtk__jl_purity_overrides_t purity;

// hidden fields:
    // lock for modifications to the method
    mmtk_jl_mutex_t writelock;
} mmtk_jl_method_t;