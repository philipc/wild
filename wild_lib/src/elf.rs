use crate::error::Result;
use anyhow::bail;
use bytemuck::Pod;
use bytemuck::Zeroable;
use object::LittleEndian;

/// Our starting address in memory when linking non-relocatable executables. We can start memory
/// addresses wherever we like, even from 0. We pick 400k because it's the same as what ld does and
/// because picking a distinctive non-zero values makes it more obvious what's happening if we mix
/// up file and memory offsets.
pub const NON_PIE_START_MEM_ADDRESS: u64 = 0x400_000;

pub(crate) type File<'data> = object::read::elf::ElfFile64<'data, LittleEndian, &'data [u8]>;
pub(crate) type Section<'data, 'file> =
    object::read::elf::ElfSection64<'data, 'file, LittleEndian, &'data [u8]>;
pub(crate) type Symbol<'data, 'file> =
    object::read::elf::ElfSymbol64<'data, 'file, LittleEndian, &'data [u8]>;
pub(crate) type SymbolIterator<'data, 'file> =
    object::read::elf::ElfSymbolIterator64<'data, 'file, LittleEndian, &'data [u8]>;
pub(crate) type FileHeader = object::elf::FileHeader64<LittleEndian>;
pub(crate) type ProgramHeader = object::elf::ProgramHeader64<LittleEndian>;
pub(crate) type SectionHeader = object::elf::SectionHeader64<LittleEndian>;
pub(crate) type SymtabEntry = object::elf::Sym64<LittleEndian>;
pub(crate) type DynamicEntry = object::elf::Dyn64<LittleEndian>;
pub(crate) type Rela = object::elf::Rela64<LittleEndian>;
pub(crate) type GnuHashHeader = object::elf::GnuHashHeader<LittleEndian>;

/// The module number for TLS variables in the current executable.
pub(crate) const CURRENT_EXE_TLS_MOD: u64 = 1;

/// Section flag bit values.
#[allow(unused)]
pub(crate) mod shf {
    pub(crate) const WRITE: u64 = 0x1;
    pub(crate) const ALLOC: u64 = 0x2;
    pub(crate) const EXECINSTR: u64 = 0x4;
    pub(crate) const MERGE: u64 = 0x10;
    pub(crate) const STRINGS: u64 = 0x20;
    pub(crate) const INFO_LINK: u64 = 0x40;
    pub(crate) const LINK_ORDER: u64 = 0x80;
    pub(crate) const OS_NONCONFORMING: u64 = 0x100;
    pub(crate) const GROUP: u64 = 0x200;
    pub(crate) const TLS: u64 = 0x400;
    pub(crate) const GNU_RETAIN: u64 = 0x200_000;
}

#[allow(unused)]
#[repr(u16)]
pub(crate) enum FileType {
    Unknown = 0,
    Relocatable = 0x1,
    Executable = 0x2,
    SharedObject = 0x3,
    CoreFile = 0x4,
}

/// Section types
#[allow(unused)]
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
#[repr(u32)]
pub(crate) enum Sht {
    #[default]
    Null = 0x0,
    Progbits = 0x1,
    Symtab = 0x2,
    Strtab = 0x3,
    Rela = 0x4,
    Hash = 0x5,
    Dynamic = 0x6,
    Note = 0x7,
    Nobits = 0x8,
    Rel = 0x9,
    Shlib = 0xa,
    DynSym = 0xb,
    InitArray = 0xe,
    FiniArray = 0xf,
    PreinitArray = 0x10,
    Group = 0x11,
    SymtabShndx = 0x12,
    Num = 0x13,
}

#[allow(unused)]
#[derive(Clone, Copy)]
#[repr(u8)]
pub(crate) enum Binding {
    Local = 0,
    Global = 1,
    Weak = 2,
}

pub(crate) const RELA_ADDRESS_OFFSET: usize = 0;
pub(crate) const RELA_ADDEND_OFFSET: usize = 16;

#[allow(unused)]
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
#[repr(u32)]
pub(crate) enum SegmentType {
    #[default]
    Null = 0,
    Load = 1,
    Dynamic = 2,
    Interp = 3,
    Note = 4,
    Shlib = 5,
    Phdr = 6,
    Tls = 7,
    EhFrame = 0x6474e550,
}

#[allow(unused)]
#[derive(Clone, Copy)]
#[repr(u64)]
pub(crate) enum DynamicTag {
    Null = 0,
    Needed = 1,
    PltRelSize = 2,
    PltGot = 3,
    Hash = 4,
    StrTab = 5,
    SymTab = 6,
    Rela = 7,
    RelaSize = 8,
    RelaEnt = 9,
    StrSize = 10,
    SymEnt = 11,
    Init = 12,
    Fini = 13,
    SoName = 14,
    Rpath = 15,
    Symbolic = 16,
    Rel = 17,
    RelSize = 18,
    RelEnt = 19,
    PltRel = 20,
    Debug = 21,
    TextRel = 22,
    JmpRel = 23,
    BindNow = 24,
    InitArray = 25,
    FiniArray = 26,
    InitArraySize = 27,
    FiniArraySize = 28,
    Flags = 30,
    GnuHash = 0x6ffffef5,
    VerSym = 0x6ffffff0,
    Flags1 = 0x6ffffffb,
    RelaCount = 0x6ffffff9,
    VerNeed = 0x6ffffffe,
    VerNeedNum = 0x6fffffff,
}

pub(crate) mod flags_1 {
    pub(crate) const NOW: u64 = 0x1;
    pub(crate) const PIE: u64 = 0x08000000;
}

pub(crate) mod flags {
    pub(crate) const BIND_NOW: u64 = 0x8;
}

/// See https://refspecs.linuxfoundation.org/LSB_1.3.0/gLSB/gLSB/ehframehdr.html
#[derive(Zeroable, Pod, Clone, Copy)]
#[repr(C)]
pub(crate) struct EhFrameHdr {
    pub(crate) version: u8,
    pub(crate) frame_pointer_encoding: u8,
    pub(crate) count_encoding: u8,
    pub(crate) table_encoding: u8,
    // For now we just use 32 bit pointer and count because it means that they're aligned. If we
    // need to upgrade these to u64, then we'd have to write these as unaligned fields.
    pub(crate) frame_pointer: i32,
    pub(crate) entry_count: u32,
}

// TODO: Use offset-of once it's stable.
pub(crate) const FRAME_POINTER_FIELD_OFFSET: usize = 4;

#[derive(Zeroable, Pod, Clone, Copy)]
#[repr(C)]
pub(crate) struct EhFrameHdrEntry {
    pub(crate) frame_ptr: i32,
    pub(crate) frame_info_ptr: i32,
}

#[derive(Zeroable, Pod, Clone, Copy)]
#[repr(C)]
pub(crate) struct EhFrameEntryPrefix {
    pub(crate) length: u32,
    pub(crate) cie_id: u32,
}

#[allow(unused)]
#[repr(u8)]
pub(crate) enum ExceptionHeaderFormat {
    Uleb128 = 1,
    U16 = 2,
    U32 = 3,
    U64 = 4,
    Sleb128 = 9,
    I16 = 0xa,
    I32 = 0xb,
    I64 = 0xc,
}

#[allow(unused)]
#[repr(u8)]
pub(crate) enum ExceptionHeaderApplication {
    Absolute = 0,

    /// Value is relative to the location of the pointer.
    Relative = 0x10,

    /// Value is relative to start of the .eh_frame_hdr section.
    EhFrameHdrRelative = 0x30,
}

#[allow(unused)]
#[derive(Clone, Copy)]
#[repr(u32)]
pub(crate) enum RelocationType {
    IRelative = 37,
}

/// The offset of the pc_begin field in an FDE.
pub(crate) const FDE_PC_BEGIN_OFFSET: usize = 8;

/// Offset in the file where we store the program headers. We always store these straight after the
/// file header.
pub(crate) const PHEADER_OFFSET: u64 = FILE_HEADER_SIZE as u64;

/// These sizes are from the spec (for 64 bit ELF).
pub(crate) const FILE_HEADER_SIZE: u16 = 0x40;
pub(crate) const PROGRAM_HEADER_SIZE: u16 = 0x38;
pub(crate) const SECTION_HEADER_SIZE: u16 = 0x40;

pub(crate) const GOT_ENTRY_SIZE: u64 = 0x8;
pub(crate) const PLT_ENTRY_SIZE: u64 = PLT_ENTRY_TEMPLATE.len() as u64;
pub(crate) const RELA_ENTRY_SIZE: u64 = 0x18;

pub(crate) const SYMTAB_ENTRY_SIZE: u64 = core::mem::size_of::<SymtabEntry>() as u64;

pub(crate) const SYMBOL_TYPE_MASK: u8 = 0xf;
pub(crate) const SYMBOL_VISIBILITY_MASK: u8 = 0xf0;

pub(crate) const SYMBOL_TYPE_IFUNC: u8 = 10;
pub(crate) const SYMBOL_TYPE_FUNC: u8 = 2;

pub(crate) const PLT_ENTRY_TEMPLATE: &[u8] = &[
    0xf3, 0x0f, 0x1e, 0xfa, // endbr64
    0xf2, 0xff, 0x25, 0x0, 0x0, 0x0, 0x0, // bnd jmp *{relative GOT address}(%rip)
    0x0f, 0x1f, 0x44, 0x0, 0x0, // nopl   0x0(%rax,%rax,1)
];

const _ASSERTS: () = {
    assert!(FILE_HEADER_SIZE as usize == std::mem::size_of::<FileHeader>());
    assert!(PROGRAM_HEADER_SIZE as usize == std::mem::size_of::<ProgramHeader>());
    assert!(SECTION_HEADER_SIZE as usize == std::mem::size_of::<SectionHeader>());
};

#[derive(Clone, Copy, Debug)]
pub(crate) enum RelocationKind {
    Absolute,
    Relative,
    Got,
    PltRelative,
    GotRelative,
    TlsGd,
    TlsLd,
    DtpOff,
    GotTpOff,
    TpOff,

    /// No relocation needs to be applied. Produced when we eliminate a relocation due to an
    /// optimisation.
    None,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RelocationKindInfo {
    pub(crate) kind: RelocationKind,
    pub(crate) byte_size: usize,
}

impl RelocationKindInfo {
    pub(crate) fn from_raw(r_type: u32) -> Result<Self> {
        let (kind, size) = match r_type {
            object::elf::R_X86_64_64 => (RelocationKind::Absolute, 8),
            object::elf::R_X86_64_PC32 => (RelocationKind::Relative, 4),
            object::elf::R_X86_64_GOT32 => (RelocationKind::Got, 4),
            object::elf::R_X86_64_PLT32 => (RelocationKind::PltRelative, 4),
            object::elf::R_X86_64_GOTPCREL => (RelocationKind::GotRelative, 4),
            object::elf::R_X86_64_32 | object::elf::R_X86_64_32S => (RelocationKind::Absolute, 4),
            object::elf::R_X86_64_16 => (RelocationKind::Absolute, 2),
            object::elf::R_X86_64_PC16 => (RelocationKind::Relative, 2),
            object::elf::R_X86_64_8 => (RelocationKind::Absolute, 1),
            object::elf::R_X86_64_PC8 => (RelocationKind::Relative, 1),
            object::elf::R_X86_64_TLSGD => (RelocationKind::TlsGd, 4),
            object::elf::R_X86_64_TLSLD => (RelocationKind::TlsLd, 4),
            object::elf::R_X86_64_DTPOFF32 => (RelocationKind::DtpOff, 4),
            object::elf::R_X86_64_GOTTPOFF => (RelocationKind::GotTpOff, 4),
            object::elf::R_X86_64_GOTPCRELX | object::elf::R_X86_64_REX_GOTPCRELX => {
                (RelocationKind::GotRelative, 4)
            }
            object::elf::R_X86_64_TPOFF32 => (RelocationKind::TpOff, 4),
            object::elf::R_X86_64_NONE => (RelocationKind::None, 0),
            _ => bail!("Unsupported relocation type {r_type}"),
        };
        Ok(Self {
            kind,
            byte_size: size,
        })
    }
}

pub(crate) fn slice_from_all_bytes_mut<T: object::Pod>(data: &mut [u8]) -> &mut [T] {
    object::slice_from_bytes_mut(data, data.len() / core::mem::size_of::<T>())
        .unwrap()
        .0
}
