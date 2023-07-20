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
