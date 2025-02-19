//! Code to double-check that we did certain things correctly. Generally only used in debug builds.

use crate::error::Result;
use crate::layout::Layout;
use crate::layout::ResolutionValue;
use crate::layout::TargetResolutionKind;
use anyhow::bail;
use anyhow::Context;
use object::Object;
use object::ObjectSection;

pub(crate) fn validate_bytes(layout: &Layout, file_bytes: &[u8]) -> Result {
    let object = crate::elf::File::parse(file_bytes).context("Failed to parse our output file")?;
    validate_object(&object, layout).context("Output validation failed")
}

/// Checks that what we actually wrote to our output file matches what we intended to write in
/// `layout`.
fn validate_object(object: &crate::elf::File, layout: &Layout) -> Result {
    if layout.args().is_relocatable() {
        // For now, we don't do any validation of relocatable outputs. The only thing we're
        // currently validating is GOT entries and they'll all have dynamic relocations.
        return Ok(());
    }
    let got = object
        .section_by_name(".got")
        .context("Missing .got from output file")?;
    let got_data = got.data()?;
    for (symbol_name, symbol_id) in &layout.symbol_db.global_names {
        match layout.symbol_resolution(*symbol_id) {
            None => {}
            Some(resolution) => {
                validate_resolution(symbol_name.bytes(), resolution, &got, got_data)?;
            }
        }
    }
    for file in &layout.file_layouts {
        match file {
            crate::layout::FileLayout::Internal(_) => {}
            crate::layout::FileLayout::Object(obj) => {
                for sec in obj.object.sections() {
                    if let Some(resolution) = obj.section_resolutions[sec.index().0] {
                        validate_resolution(sec.name_bytes()?, &resolution, &got, got_data)?;
                    }
                }
            }
            crate::layout::FileLayout::Dynamic(_) => {}
            crate::layout::FileLayout::Epilogue(_) => {}
            crate::layout::FileLayout::NotLoaded => {}
        }
    }
    Ok(())
}

fn validate_resolution(
    name: &[u8],
    resolution: &crate::layout::Resolution,
    got: &crate::elf::Section,
    got_data: &[u8],
) -> Result {
    let res_kind = resolution.kind;
    if matches!(
        res_kind,
        TargetResolutionKind::IFunc | TargetResolutionKind::GotTlsOffset
    ) {
        return Ok(());
    };
    if let Some(got_address) = resolution.got_address {
        let start_offset = (got_address.get() - got.address()) as usize;
        let end_offset = start_offset + core::mem::size_of::<u64>();
        if end_offset > got_data.len() {
            bail!("GOT offset beyond end of GOT 0x{end_offset}");
        }
        let expected = match resolution.value {
            ResolutionValue::Absolute(v) | ResolutionValue::Address(v) => v,
            ResolutionValue::Dynamic(_) => return Ok(()),
        };
        let address = bytemuck::pod_read_unaligned(&got_data[start_offset..end_offset]);
        if expected != address {
            let name = String::from_utf8_lossy(name);
            bail!(
                "res={res_kind:?} `{name}` has address 0x{expected:x}, but GOT \
                 (at 0x{got_address:x}) points to 0x{address:x}"
            );
        }
    }
    Ok(())
}
