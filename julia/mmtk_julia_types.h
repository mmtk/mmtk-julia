// install bindgen with cargo install bindgen-cli
// run ~/.cargo/bin/bindgen /home/eduardo/mmtk-julia/julia/mmtk_julia_types.h -o /home/eduardo/mmtk-julia/mmtk/src/julia_types.rs
#include <setjmp.h>	

typedef signed char __int8_t;
typedef unsigned char __uint8_t;
typedef signed short int __int16_t;
typedef unsigned short int __uint16_t;
typedef signed int __int32_t;
typedef unsigned int __uint32_t;
typedef signed long int __int64_t;
typedef unsigned long int __uint64_t;

typedef __SIZE_TYPE__ size_t;

typedef __int8_t int8_t;
typedef __int16_t int16_t;
typedef __int32_t int32_t;
typedef __int64_t int64_t;
typedef __uint8_t uint8_t;
typedef __uint16_t uint16_t;
typedef __uint32_t uint32_t;
typedef __uint64_t uint64_t;

/* Types for `void *' pointers.  */
typedef int			intptr_t;
typedef unsigned long int	uintptr_t;

struct mmtk__jl_taggedvalue_bits {
    uintptr_t gc:2;
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
    uint8_t max_methods; // override for inference's max_methods setting (0 = no additional limit or relaxation)
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
    int32_t size; // TODO: move to _jl_datatype_layout_t
    // memoized properties
    uint32_t hash;
    uint8_t hasfreetypevars:1; // majority part of isconcrete computation
    uint8_t isconcretetype:1; // whether this type can have instances
    uint8_t isdispatchtuple:1; // aka isleaftupletype
    uint8_t isbitstype:1; // relevant query for C-api and type-parameters
    uint8_t zeroinit:1; // if one or more fields requires zero-initialization
    uint8_t has_concrete_subtype:1; // If clear, no value will have this datatype
    uint8_t cached_by_hash:1; // stored in hash-based set cache (instead of linear cache)
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
    mmtk_jl_sym_t *name;
    _Atomic(void*) value;
    _Atomic(void*) globalref;  // cached GlobalRef for this binding
    struct mmtk__jl_module_t* owner;  // for individual imported bindings -- TODO: make _Atomic
    _Atomic(void*) ty;  // binding type
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
    uint64_t build_id;
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
typedef struct __jmp_buf_tag sigjmp_buf[1];

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

#ifndef _OS_WINDOWS_
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
#else
#  include "win32_ucontext.h"
#  define mmtk_jl_jmp_buf jmp_buf
#  define MAX_ALIGN 8
#endif

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
} mmtk_jl_ucontext_t;

typedef struct mmtk__jl_gcframe_t mmtk_jl_gcframe_t;

struct mmtk__jl_gcframe_t {
    size_t nroots;
    struct mmtk__jl_gcframe_t *prev;
    // actual roots go here
};

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
    unsigned int copy_stack:31; // sizeof stack for copybuf
    unsigned int started:1;
} mmtk_jl_task_t;

typedef struct {
    mmtk_jl_value_t *value;
} mmtk_jl_weakref_t;
