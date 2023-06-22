use crate::{CpuStorage, CudaStorage, DType, Device, Error, Result, Shape};

#[derive(Debug, Clone)]
pub enum Storage {
    Cpu(CpuStorage),
    Cuda(CudaStorage),
}

pub(crate) trait UnaryOp {
    const NAME: &'static str;
    fn f32(v1: f32) -> f32;
    fn f64(v1: f64) -> f64;
}

pub(crate) trait BinaryOp {
    const NAME: &'static str;
    const KERNEL_F32: &'static str;
    const KERNEL_F64: &'static str;
    fn f32(v1: f32, v2: f32) -> f32;
    fn f64(v1: f64, v2: f64) -> f64;
}

struct Add;
struct Div;
struct Mul;
struct Sub;
struct Neg;
struct Sqr;
struct Sqrt;

impl BinaryOp for Add {
    const NAME: &'static str = "add";
    const KERNEL_F32: &'static str = "badd_f32";
    const KERNEL_F64: &'static str = "badd_f64";
    fn f32(v1: f32, v2: f32) -> f32 {
        v1 + v2
    }
    fn f64(v1: f64, v2: f64) -> f64 {
        v1 + v2
    }
}

impl BinaryOp for Sub {
    const NAME: &'static str = "sub";
    const KERNEL_F32: &'static str = "bsub_f32";
    const KERNEL_F64: &'static str = "bsub_f64";
    fn f32(v1: f32, v2: f32) -> f32 {
        v1 - v2
    }
    fn f64(v1: f64, v2: f64) -> f64 {
        v1 - v2
    }
}

impl BinaryOp for Mul {
    const NAME: &'static str = "mul";
    const KERNEL_F32: &'static str = "bmul_f32";
    const KERNEL_F64: &'static str = "bmul_f64";
    fn f32(v1: f32, v2: f32) -> f32 {
        v1 * v2
    }
    fn f64(v1: f64, v2: f64) -> f64 {
        v1 * v2
    }
}

impl BinaryOp for Div {
    const NAME: &'static str = "div";
    const KERNEL_F32: &'static str = "bdiv_f32";
    const KERNEL_F64: &'static str = "bdiv_f64";
    fn f32(v1: f32, v2: f32) -> f32 {
        v1 / v2
    }
    fn f64(v1: f64, v2: f64) -> f64 {
        v1 / v2
    }
}

impl UnaryOp for Neg {
    const NAME: &'static str = "neg";
    fn f32(v1: f32) -> f32 {
        -v1
    }
    fn f64(v1: f64) -> f64 {
        -v1
    }
}

impl UnaryOp for Sqr {
    const NAME: &'static str = "sqr";
    fn f32(v1: f32) -> f32 {
        v1 * v1
    }
    fn f64(v1: f64) -> f64 {
        v1 * v1
    }
}

impl UnaryOp for Sqrt {
    const NAME: &'static str = "sqrt";
    fn f32(v1: f32) -> f32 {
        v1.sqrt()
    }
    fn f64(v1: f64) -> f64 {
        v1.sqrt()
    }
}

impl Storage {
    pub fn device(&self) -> Device {
        match self {
            Self::Cpu(_) => Device::Cpu,
            Self::Cuda(storage) => Device::Cuda(storage.device()),
        }
    }

    pub fn dtype(&self) -> DType {
        match self {
            Self::Cpu(storage) => storage.dtype(),
            Self::Cuda(storage) => storage.dtype(),
        }
    }

    pub(crate) fn same_device(&self, rhs: &Self, op: &'static str) -> Result<()> {
        let lhs = self.device().location();
        let rhs = rhs.device().location();
        if lhs != rhs {
            Err(Error::DeviceMismatchBinaryOp { lhs, rhs, op })
        } else {
            Ok(())
        }
    }

    pub(crate) fn same_dtype(&self, rhs: &Self, op: &'static str) -> Result<()> {
        let lhs = self.dtype();
        let rhs = rhs.dtype();
        if lhs != rhs {
            Err(Error::DTypeMismatchBinaryOp { lhs, rhs, op })
        } else {
            Ok(())
        }
    }

    pub(crate) fn affine_impl(
        &self,
        shape: &Shape,
        stride: &[usize],
        mul: f64,
        add: f64,
    ) -> Result<Self> {
        // TODO: Different code path for the contiguous case?
        match self {
            Storage::Cpu(storage) => {
                let storage = storage.affine_impl(shape, stride, mul, add)?;
                Ok(Self::Cpu(storage))
            }
            Self::Cuda(storage) => {
                let storage = storage.affine_impl(shape, stride, mul, add)?;
                Ok(Self::Cuda(storage))
            }
        }
    }

    fn unary_impl<B: UnaryOp>(&self, shape: &Shape, stride: &[usize]) -> Result<Self> {
        // TODO: Different code path for the contiguous case?
        match self {
            Storage::Cpu(storage) => {
                let storage = storage.unary_impl::<B>(shape, stride)?;
                Ok(Self::Cpu(storage))
            }
            Self::Cuda { .. } => todo!(),
        }
    }

    // TODO: Support broadcasting?
    fn binary_impl<B: BinaryOp>(
        &self,
        rhs: &Self,
        shape: &Shape,
        lhs_stride: &[usize],
        rhs_stride: &[usize],
    ) -> Result<Self> {
        self.same_device(rhs, B::NAME)?;
        self.same_dtype(rhs, B::NAME)?;
        match (self, rhs) {
            (Storage::Cpu(lhs), Storage::Cpu(rhs)) => {
                let storage = lhs.binary_impl::<B>(rhs, shape, lhs_stride, rhs_stride)?;
                Ok(Self::Cpu(storage))
            }
            (Self::Cuda(lhs), Self::Cuda(rhs)) => {
                let storage = lhs.binary_impl::<B>(rhs, shape, lhs_stride, rhs_stride)?;
                Ok(Self::Cuda(storage))
            }
            (lhs, rhs) => {
                // Should not happen because of the same device check above but we're defensive
                // anyway.
                Err(Error::DeviceMismatchBinaryOp {
                    lhs: lhs.device().location(),
                    rhs: rhs.device().location(),
                    op: B::NAME,
                })
            }
        }
    }

    pub(crate) fn add_impl(
        &self,
        rhs: &Self,
        shape: &Shape,
        lhs_stride: &[usize],
        rhs_stride: &[usize],
    ) -> Result<Self> {
        self.binary_impl::<Add>(rhs, shape, lhs_stride, rhs_stride)
    }

    pub(crate) fn sub_impl(
        &self,
        rhs: &Self,
        shape: &Shape,
        lhs_stride: &[usize],
        rhs_stride: &[usize],
    ) -> Result<Self> {
        self.binary_impl::<Sub>(rhs, shape, lhs_stride, rhs_stride)
    }

    pub(crate) fn mul_impl(
        &self,
        rhs: &Self,
        shape: &Shape,
        lhs_stride: &[usize],
        rhs_stride: &[usize],
    ) -> Result<Self> {
        self.binary_impl::<Mul>(rhs, shape, lhs_stride, rhs_stride)
    }

    pub(crate) fn div_impl(
        &self,
        rhs: &Self,
        shape: &Shape,
        lhs_stride: &[usize],
        rhs_stride: &[usize],
    ) -> Result<Self> {
        self.binary_impl::<Div>(rhs, shape, lhs_stride, rhs_stride)
    }

    pub(crate) fn neg_impl(&self, shape: &Shape, stride: &[usize]) -> Result<Self> {
        self.unary_impl::<Neg>(shape, stride)
    }

    pub(crate) fn sqr_impl(&self, shape: &Shape, stride: &[usize]) -> Result<Self> {
        self.unary_impl::<Sqr>(shape, stride)
    }

    pub(crate) fn sqrt_impl(&self, shape: &Shape, stride: &[usize]) -> Result<Self> {
        self.unary_impl::<Sqrt>(shape, stride)
    }

    pub(crate) fn matmul_impl(
        &self,
        rhs: &Self,
        bmnk: (usize, usize, usize, usize),
        lhs_stride: &[usize],
        rhs_stride: &[usize],
    ) -> Result<Self> {
        self.same_device(rhs, "matmul")?;
        self.same_dtype(rhs, "matmul")?;
        match (self, rhs) {
            (Storage::Cpu(storage), Storage::Cpu(rhs_storage)) => {
                let storage = storage.matmul_impl(rhs_storage, bmnk, lhs_stride, rhs_stride)?;
                Ok(Self::Cpu(storage))
            }
            _ => todo!(),
        }
    }
}
