use crate::{init_boot_image_metadata_info, JuliaVM, UPCALLS};
use mmtk::util::constants::BYTES_IN_PAGE;
use mmtk::util::copy::*;
use mmtk::util::metadata::side_metadata::{
    SideMetadataContext, SideMetadataOffset, SideMetadataSpec,
};
use mmtk::util::{Address, ObjectReference};
use mmtk::vm::ObjectModel;
use mmtk::vm::*;

pub struct VMObjectModel {}

/// Global logging bit metadata spec
/// 1 bit per object
pub(crate) const LOGGING_SIDE_METADATA_SPEC: VMGlobalLogBitSpec = VMGlobalLogBitSpec::side_first();

pub(crate) const MARKING_METADATA_SPEC: VMLocalMarkBitSpec =
    VMLocalMarkBitSpec::side_after(LOS_METADATA_SPEC.as_spec());

// pub(crate) const LOCAL_FORWARDING_POINTER_METADATA_SPEC: VMLocalForwardingPointerSpec =
//     VMLocalForwardingPointerSpec::side_after(MARKING_METADATA_SPEC.as_spec());

// pub(crate) const LOCAL_FORWARDING_METADATA_BITS_SPEC: VMLocalForwardingBitsSpec =
//     VMLocalForwardingBitsSpec::side_after(LOCAL_FORWARDING_POINTER_METADATA_SPEC.as_spec());

pub(crate) const BI_MARKING_METADATA_SPEC: SideMetadataSpec = SideMetadataSpec {
    name: "BI_MARK",
    is_global: false,
    offset: SideMetadataOffset::layout_after(MARKING_METADATA_SPEC.as_spec().extract_side_spec()),
    log_num_of_bits: 0,
    log_bytes_in_region: 3,
};

lazy_static! {
    pub static ref BI_METADATA_CONTEXT: SideMetadataContext = SideMetadataContext {
        global: vec![],
        local: vec![BI_MARKING_METADATA_SPEC],
    };
}

/// PolicySpecific mark-and-nursery bits metadata spec
/// 2-bits per object
pub(crate) const LOS_METADATA_SPEC: VMLocalLOSMarkNurserySpec =
    VMLocalLOSMarkNurserySpec::side_first();

impl ObjectModel<JuliaVM> for VMObjectModel {
    const GLOBAL_LOG_BIT_SPEC: VMGlobalLogBitSpec = LOGGING_SIDE_METADATA_SPEC;
    const LOCAL_FORWARDING_POINTER_SPEC: VMLocalForwardingPointerSpec =
        VMLocalForwardingPointerSpec::in_header(0);
    const LOCAL_FORWARDING_BITS_SPEC: VMLocalForwardingBitsSpec =
        VMLocalForwardingBitsSpec::in_header(0);
    const LOCAL_MARK_BIT_SPEC: VMLocalMarkBitSpec = MARKING_METADATA_SPEC;
    const LOCAL_LOS_MARK_NURSERY_SPEC: VMLocalLOSMarkNurserySpec = LOS_METADATA_SPEC;
    const UNIFIED_OBJECT_REFERENCE_ADDRESS: bool = false;
    const OBJECT_REF_OFFSET_LOWER_BOUND: isize = 0;

    fn copy(
        _from: ObjectReference,
        _semantics: CopySemantics,
        _copy_context: &mut GCWorkerCopyContext<JuliaVM>,
    ) -> ObjectReference {
        unimplemented!()
    }

    fn copy_to(_from: ObjectReference, _to: ObjectReference, _region: Address) -> Address {
        unimplemented!()
    }

    fn get_current_size(object: ObjectReference) -> usize {
        let size = if is_object_in_los(&object) {
            unsafe { ((*UPCALLS).get_lo_size)(object) }
        } else {
            let obj_size = unsafe { ((*UPCALLS).get_so_size)(object) };
            obj_size
        };

        size as usize
    }

    fn get_size_when_copied(_object: ObjectReference) -> usize {
        unimplemented!()
    }

    fn get_align_when_copied(_object: ObjectReference) -> usize {
        unimplemented!()
    }

    fn get_align_offset_when_copied(_object: ObjectReference) -> isize {
        unimplemented!()
    }

    fn get_reference_when_copied_to(_from: ObjectReference, _to: Address) -> ObjectReference {
        unimplemented!()
    }

    fn get_type_descriptor(_reference: ObjectReference) -> &'static [i8] {
        unimplemented!()
    }

    #[inline(always)]
    fn ref_to_object_start(object: ObjectReference) -> Address {
        let res = if is_object_in_los(&object) {
            object.to_raw_address() - 48
        } else {
            unsafe { ((*UPCALLS).get_object_start_ref)(object) }
        };
        res
    }

    #[inline(always)]
    fn ref_to_address(object: ObjectReference) -> Address {
        object.to_raw_address()
    }

    #[inline(always)]
    fn address_to_ref(address: Address) -> ObjectReference {
        ObjectReference::from_raw_address(address)
    }

    #[inline(always)]
    fn ref_to_header(object: ObjectReference) -> Address {
        object.to_raw_address()
    }

    fn dump_object(_object: ObjectReference) {
        unimplemented!()
    }
}

pub fn is_object_in_los(object: &ObjectReference) -> bool {
    (*object).to_raw_address().as_usize() > 0x60000000000
}

#[no_mangle]
pub extern "C" fn map_boot_image_metadata(start: Address, end: Address) {
    let start_address_aligned_down = start.align_down(BYTES_IN_PAGE);
    let end_address_aligned_up = end.align_up(BYTES_IN_PAGE);
    unsafe {
        init_boot_image_metadata_info(
            start_address_aligned_down.as_usize(),
            end_address_aligned_up.as_usize(),
        );
    }
    let res = BI_METADATA_CONTEXT.try_map_metadata_space(
        start_address_aligned_down,
        end_address_aligned_up.as_usize() - start_address_aligned_down.as_usize(),
    );

    match res {
        Ok(_) => (),
        Err(e) => panic!("Mapping failed with error {}", e),
    }
}
