use enum_map::Enum;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Enum, PartialEq, Hash, Eq)]
pub enum RootLabel {
    MarkAndScan = 0,
    ScanOnly = 1,
    FinList = 2,
    ObjArray = 3,
    Array8 = 4,
    Obj8 = 5,
    Obj16 = 6,
    Obj32 = 7,
    Stack = 8,
    ExcStack = 9,
    ModuleBinding = 10,
}

impl RootLabel {
    pub fn from_u32(value: u32) -> RootLabel {
        match value {
            0 => RootLabel::MarkAndScan,
            1 => RootLabel::ScanOnly,
            2 => RootLabel::FinList,
            3 => RootLabel::ObjArray,
            4 => RootLabel::Array8,
            5 => RootLabel::Obj8,
            6 => RootLabel::Obj16,
            7 => RootLabel::Obj32,
            8 => RootLabel::Stack,
            9 => RootLabel::ExcStack,
            10 => RootLabel::ModuleBinding,
            _ => panic!("Unknown value: {}", value),
        }
    }
}

const PRINT_STRUCT_SIZE: bool = false;

macro_rules! print_sizeof {
    ($t: ty) => {
        {
            let sz = std::mem::size_of::<$t>();
            if PRINT_STRUCT_SIZE {
                println!("Rust {} = {} bytes", stringify!($t), sz);
            }
            sz
        }
    };
}

pub(crate) fn get_abi_structs_checksum_rust() -> usize {
    use crate::julia_types::*;
    print_sizeof!(mmtk::Mutator<crate::JuliaVM>)
        ^ print_sizeof!(mmtk__jl_taggedvalue_bits)
        ^ print_sizeof!(mmtk_jl_taggedvalue_t)
        ^ print_sizeof!(mmtk_jl_array_flags_t)
        ^ print_sizeof!(mmtk_jl_datatype_layout_t)
        ^ print_sizeof!(mmtk_jl_typename_t)
        ^ print_sizeof!(mmtk_jl_svec_t)
        ^ print_sizeof!(mmtk_jl_datatype_t)
        ^ print_sizeof!(mmtk_jl_array_t)
        ^ print_sizeof!(mmtk_jl_sym_t)
        ^ print_sizeof!(mmtk_jl_binding_t)
        ^ print_sizeof!(mmtk_htable_t)
        ^ print_sizeof!(mmtk_arraylist_t)
        ^ print_sizeof!(mmtk_jl_uuid_t)
        ^ print_sizeof!(mmtk_jl_mutex_t)
        ^ print_sizeof!(mmtk_jl_module_t)
        ^ print_sizeof!(mmtk_jl_excstack_t)
        ^ print_sizeof!(mmtk_jl_bt_element_t)
        ^ print_sizeof!(mmtk_jl_stack_context_t)
        ^ print_sizeof!(mmtk_jl_ucontext_t)
        ^ print_sizeof!(mmtk__jl_gcframe_t)
        ^ print_sizeof!(mmtk_jl_task_t)
        ^ print_sizeof!(mmtk_jl_weakref_t)
}
