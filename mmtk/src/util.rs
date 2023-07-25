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

pub(crate) fn get_abi_structs_checksum_rust() -> usize {
    use std::mem;
    return mem::size_of::<mmtk::Mutator<crate::JuliaVM>>();
}

// The functions below allow accessing the values of bitfields without performing a for loop
use crate::julia_types::{
    __BindgenBitfieldUnit, mmtk__jl_task_t, mmtk_jl_array_flags_t, mmtk_jl_datatype_layout_t,
};

impl mmtk_jl_datatype_layout_t {
    #[inline]
    pub fn fielddesc_type_custom(&self) -> u16 {
        let fielddesc_type_raw: u16 = unsafe {
            ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 2usize]>, u16>(self._bitfield_1)
        };
        fielddesc_type_raw >> 1 & 0b11
    }
}

impl mmtk_jl_array_flags_t {
    #[inline]
    pub fn how_custom(&self) -> u16 {
        let how_raw: u16 = unsafe {
            ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 2usize]>, u16>(self._bitfield_1)
        };
        how_raw & 0b11
    }
    #[inline]
    pub fn ndims_custom(&self) -> u16 {
        let ndims_raw: u16 = unsafe {
            ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 2usize]>, u16>(self._bitfield_1)
        };
        ndims_raw >> 2 & 0b111_111_111
    }
    #[inline]
    pub fn ptrarray_custom(&self) -> u16 {
        let ptrarray_raw: u16 = unsafe {
            ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 2usize]>, u16>(self._bitfield_1)
        };
        ptrarray_raw >> 12 & 0b1
    }
    #[inline]
    pub fn hasptr_custom(&self) -> u16 {
        let hasptr_raw: u16 = unsafe {
            ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 2usize]>, u16>(self._bitfield_1)
        };
        hasptr_raw >> 13 & 0b1
    }
}

impl mmtk__jl_task_t {
    #[inline]
    pub fn copy_stack_custom(&self) -> u32 {
        let copy_stack_raw: u32 = unsafe {
            ::std::mem::transmute::<__BindgenBitfieldUnit<[u8; 4usize]>, u32>(self._bitfield_1)
        };
        copy_stack_raw & 2147483647u32
    }
}
