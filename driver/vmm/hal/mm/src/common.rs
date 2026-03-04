use core::marker::{ConstParamTy, PhantomData};

pub const HHDM_OFFSET: u64 = 0x8000000;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PhysWidth(u8);

impl PhysWidth {
    pub fn new(width: u8) -> Self {
        // Chose 36 as the minimum since thats what's required for PAE.
        // TODO: relook at this limit
        assert!(
            (36..=52).contains(&width),
            "Physical width {} is out of x86_64 boundaries.",
            width
        );
        Self(width)
    }

    #[inline(always)]
    pub fn is_pfn_valid(&self, pfn: u64) -> bool {
        pfn < (1u64 << (self.0 - 12))
    }

    #[inline(always)]
    pub fn solely_pfn_mask(&self) -> u64 {
        (1u64 << (self.0 - 12)) - 1
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, ConstParamTy)]
pub enum PagingLevel {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, ConstParamTy)]
pub enum PagingMode {
    FourLevel = 48,
    FiveLevel = 57,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PhysAddr<T> {
    pfn: u64,
    offset: u16,
    width: PhysWidth,
    phantom: PhantomData<T>,
}

// Since Clone for derive requires PhantomData<T> to be Clone
impl<T> Clone for PhysAddr<T> {
    fn clone(&self) -> Self {
        Self {
            pfn: self.pfn,
            offset: self.offset,
            width: self.width,
            phantom: PhantomData,
        }
    }
}

impl<T> PhysAddr<T> {
    pub fn new(pfn: u64, offset: u16, width: PhysWidth) -> Self {
        // TODO: this souldnt be here
        assert!(width.is_pfn_valid(pfn));

        let offset = offset & 0xFFF;
        Self {
            pfn,
            offset,
            width,
            phantom: PhantomData,
        }
    }

    pub fn cast<U>(self) -> PhysAddr<U> {
        PhysAddr {
            pfn: self.pfn,
            offset: self.offset,
            width: self.width,
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn pfn(&self) -> u64 {
        self.pfn
    }

    #[inline(always)]
    pub fn set_pfn(&mut self, pfn: u64) {
        self.pfn = pfn;
    }

    #[inline(always)]
    pub fn offset(&self) -> u16 {
        self.offset
    }

    #[inline(always)]
    pub fn width(&self) -> PhysWidth {
        self.width
    }

    #[inline(always)]
    pub fn as_address(&self) -> u64 {
        (self.pfn << 12) | (self.offset as u64)
    }

    // Safety: Assumes identity mapped address space.
    #[inline(always)]
    pub unsafe fn as_im_virt<const M: PagingMode>(self) -> VirtAddr<M, T> {
        VirtAddr::<M, T>::new(self.pfn, self.offset)
    }

    #[inline(always)]
    pub unsafe fn as_hhdm_virt<const M: PagingMode>(self) -> VirtAddr<M, T> {
        // TODO: overflolws
        VirtAddr::<M, T>::new(HHDM_OFFSET + self.pfn, self.offset)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VirtAddr<const M: PagingMode, T> {
    vpn: u64,
    offset: u16,
    phantom: PhantomData<T>,
}

impl<const M: PagingMode, T> VirtAddr<M, T> {
    pub fn new(vpn: u64, offset: u16) -> Self {
        // TODO: This should be able to fail if the VPN is too big.
        let vpn_mask = (1u64 << ((M as u8) - 12)) - 1;
        let vpn = vpn & vpn_mask;

        let offset = offset & 0xFFF;

        Self {
            vpn,
            offset,
            phantom: PhantomData,
        }
    }

    pub fn cast<U>(self) -> VirtAddr<M, U> {
        VirtAddr {
            vpn: self.vpn,
            offset: self.offset,
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn vpn(&self) -> u64 {
        self.vpn
    }

    #[inline(always)]
    pub fn set_vpn(&mut self, vpn: u64) {
        self.vpn = vpn;
    }

    #[inline(always)]
    pub fn offset(&self) -> u16 {
        self.offset
    }

    #[inline(always)]
    pub fn as_canonical_address(&self) -> u64 {
        let addr = (self.vpn << 12) | (self.offset as u64);
        let shift = 64 - (M as u8);
        ((addr << shift) as i64 >> shift) as u64
    }

    #[inline(always)]
    pub fn as_ptr(self) -> *const T {
        self.as_canonical_address() as *const T
    }

    #[inline(always)]
    pub fn as_mut_ptr(self) -> *mut T {
        self.as_canonical_address() as *mut T
    }

    #[inline(always)]
    pub fn level_index<const L: PagingLevel>(&self) -> usize {
        const {
            assert!(!match (L, M) {
                (PagingLevel::Five, PagingMode::FiveLevel) => false,
                (PagingLevel::Five, _) => true,
                _ => false,
            });
        }

        let shift = ((L as u8) - 1) * 9;
        ((self.vpn >> shift) & 0x1FF) as usize
    }
}
