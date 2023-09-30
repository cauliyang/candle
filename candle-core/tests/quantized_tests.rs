use candle_core::{
    quantized::{self, GgmlDType},
    test_utils::to_vec2_round,
    Device, Result, Tensor,
};
use quantized::{k_quants, GgmlType};
use rand::prelude::*;

const GGML_TEST_SIZE: usize = 32 * 128;

const GGML_MAX_QUANTIZATION_TOTAL_ERROR: f32 = 0.002;
const GGML_MAX_QUANTIZATION_TOTAL_ERROR_2BITS: f32 = 0.0075;
const GGML_MAX_QUANTIZATION_TOTAL_ERROR_3BITS: f32 = 0.0040;
const GGML_MAX_DOT_PRODUCT_ERROR: f32 = 0.02;

#[test]
fn quantized_matmul() -> Result<()> {
    let cpu = &Device::Cpu;
    let (m, k, n) = (3, 64, 4);
    let lhs = (0..(m * k)).map(|v| v as f32).collect::<Vec<_>>();
    let tensor_lhs = Tensor::from_slice(&lhs, (m, k), cpu)?;
    let mut dst = vec![42.; 3 * 4];
    let mut rhs_t = vec![k_quants::BlockQ4_0::zeros(); 8];
    let rhs = (0..(k * n)).map(|v| v as f32).collect::<Vec<_>>();
    let tensor_rhs = Tensor::from_slice(&rhs, (n, k), cpu)?.t()?;
    k_quants::BlockQ4_0::from_float(&rhs, &mut rhs_t)?;
    k_quants::matmul((m, k, n), &lhs, &rhs_t, &mut dst)?;
    assert_eq!(
        dst.iter().map(|x| x.round()).collect::<Vec<_>>(),
        &[
            85120.0, 214562.0, 345455.0, 474748.0, 213475.0, 604465.0, 1000686.0, 1388317.0,
            341876.0, 994283.0, 1655709.0, 2301518.0
        ]
    );
    let mm = tensor_lhs.matmul(&tensor_rhs)?;
    assert_eq!(
        mm.to_vec2::<f32>()?,
        &[
            [85344.0, 214368.0, 343392.0, 472416.0],
            [214368.0, 605536.0, 996704.0, 1387872.0],
            [343392.0, 996704.0, 1650016.0, 2303328.0]
        ]
    );

    let qtensor = quantized::QTensor::new(rhs_t, (4, 64))?;
    let matmul = quantized::QMatMul::from_qtensor(qtensor);
    let res = matmul.forward(&tensor_lhs)?;
    assert_eq!(
        to_vec2_round(&res, 0)?,
        &[
            [85120.0, 214562.0, 345455.0, 474748.0],
            [213475.0, 604465.0, 1000686.0, 1388317.0],
            [341876.0, 994283.0, 1655709.0, 2301518.0]
        ]
    );

    Ok(())
}

#[test]
fn quantized_matmul_neg() -> Result<()> {
    let cpu = &Device::Cpu;
    let (m, k, n) = (3, 64, 4);
    let lhs = (0..(m * k))
        .map(|v| v as f32 - (m * k) as f32 / 2.0)
        .collect::<Vec<_>>();
    let tensor_lhs = Tensor::from_slice(&lhs, (m, k), cpu)?;
    let mut dst = vec![42.; 3 * 4];
    let mut rhs_t = vec![k_quants::BlockQ4_0::zeros(); 8];
    let rhs = (0..k * n)
        .map(|v| v as f32 - (k * n) as f32 / 3.0)
        .collect::<Vec<_>>();
    let tensor_rhs = Tensor::from_slice(&rhs, (n, k), cpu)?.t()?;
    k_quants::BlockQ4_0::from_float(&rhs, &mut rhs_t)?;
    k_quants::matmul((m, k, n), &lhs, &rhs_t, &mut dst)?;
    assert_eq!(
        dst.iter().map(|x| x.round()).collect::<Vec<_>>(),
        &[
            243524.0, -19596.0, -285051.0, -549815.0, 23777.0, 21651.0, 19398.0, 18367.0,
            -196472.0, 63012.0, 324585.0, 587902.0
        ]
    );
    let mm = tensor_lhs.matmul(&tensor_rhs)?;
    assert_eq!(
        to_vec2_round(&mm, 0)?,
        &[
            [244064.0, -20128.0, -284320.0, -548512.0],
            [23563.0, 21515.0, 19467.0, 17419.0],
            [-196939.0, 63157.0, 323253.0, 583349.0]
        ]
    );

    let qtensor = quantized::QTensor::new(rhs_t, (4, 64))?;
    let matmul = quantized::QMatMul::from_qtensor(qtensor);
    let res = matmul.forward(&tensor_lhs)?;
    assert_eq!(
        to_vec2_round(&res, 0)?,
        &[
            [243524.0, -19596.0, -285051.0, -549815.0],
            [23777.0, 21651.0, 19398.0, 18367.0],
            [-196472.0, 63012.0, 324585.0, 587902.0]
        ]
    );

    Ok(())
}

#[test]
fn quantize_q4_0() -> Result<()> {
    use k_quants::BlockQ4_0;

    let src = (0..32 * 4).map(|v| v as f32).collect::<Vec<_>>();
    let mut dst = vec![0f32; 32 * 4];
    let mut quant = vec![BlockQ4_0::zeros(); 4];
    BlockQ4_0::from_float(&src, &mut quant)?;
    BlockQ4_0::to_float(&quant, dst.as_mut_slice())?;
    assert_eq!(
        dst,
        &[
            -0.0, -0.0, 3.875, 3.875, 3.875, 3.875, 7.75, 7.75, 7.75, 7.75, 11.625, 11.625, 11.625,
            11.625, 15.5, 15.5, 15.5, 15.5, 19.375, 19.375, 19.375, 19.375, 23.25, 23.25, 23.25,
            23.25, 27.125, 27.125, 27.125, 27.125, 31.0, 31.0, 31.5, 31.5, 31.5, 31.5, 39.375,
            39.375, 39.375, 39.375, 39.375, 39.375, 39.375, 39.375, 47.25, 47.25, 47.25, 47.25,
            47.25, 47.25, 47.25, 47.25, 55.125, 55.125, 55.125, 55.125, 55.125, 55.125, 55.125,
            55.125, 63.0, 63.0, 63.0, 63.0, 59.375, 59.375, 71.25, 71.25, 71.25, 71.25, 71.25,
            71.25, 71.25, 71.25, 71.25, 71.25, 71.25, 71.25, 83.125, 83.125, 83.125, 83.125,
            83.125, 83.125, 83.125, 83.125, 83.125, 83.125, 83.125, 83.125, 95.0, 95.0, 95.0, 95.0,
            95.0, 95.0, 95.25, 95.25, 95.25, 95.25, 95.25, 95.25, 95.25, 95.25, 111.125, 111.125,
            111.125, 111.125, 111.125, 111.125, 111.125, 111.125, 111.125, 111.125, 111.125,
            111.125, 111.125, 111.125, 111.125, 111.125, 127.0, 127.0, 127.0, 127.0, 127.0, 127.0,
            127.0, 127.0
        ]
    );
    ggml_quantization_error_test::<BlockQ4_0>(GGML_MAX_QUANTIZATION_TOTAL_ERROR)?;
    Ok(())
}

#[test]
fn quantize_q4_1() -> Result<()> {
    use k_quants::BlockQ4_1;

    let src = (0..32 * 4).map(|v| v as f32).collect::<Vec<_>>();
    let mut dst = vec![0f32; 32 * 4];
    let mut quant = vec![BlockQ4_1::zeros(); 4];
    BlockQ4_1::from_float(&src, &mut quant)?;
    BlockQ4_1::to_float(&quant, dst.as_mut_slice())?;
    assert_eq!(
        round_vector(&dst),
        &[
            0.0, 0.0, 2.066, 2.066, 4.133, 4.133, 6.199, 6.199, 8.266, 8.266, 10.332, 10.332,
            12.398, 12.398, 14.465, 14.465, 16.531, 16.531, 18.598, 18.598, 20.664, 20.664, 22.73,
            22.73, 24.797, 24.797, 26.863, 26.863, 28.93, 28.93, 30.996, 30.996, 32.0, 32.0,
            34.066, 34.066, 36.133, 36.133, 38.199, 38.199, 40.266, 40.266, 42.332, 42.332, 44.398,
            44.398, 46.465, 46.465, 48.531, 48.531, 50.598, 50.598, 52.664, 52.664, 54.73, 54.73,
            56.797, 56.797, 58.863, 58.863, 60.93, 60.93, 62.996, 62.996, 64.0, 64.0, 66.066,
            66.066, 68.133, 68.133, 70.199, 70.199, 72.266, 72.266, 74.332, 74.332, 76.398, 76.398,
            78.465, 78.465, 80.531, 80.531, 82.598, 82.598, 84.664, 84.664, 86.73, 86.73, 88.797,
            88.797, 90.863, 90.863, 92.93, 92.93, 94.996, 94.996, 96.0, 96.0, 98.066, 98.066,
            100.133, 100.133, 102.199, 102.199, 104.266, 104.266, 106.332, 106.332, 108.398,
            108.398, 110.465, 110.465, 112.531, 112.531, 114.598, 114.598, 116.664, 116.664,
            118.73, 118.73, 120.797, 120.797, 122.863, 122.863, 124.93, 124.93, 126.996, 126.996
        ]
    );
    ggml_quantization_error_test::<BlockQ4_1>(GGML_MAX_QUANTIZATION_TOTAL_ERROR)?;
    Ok(())
}

#[test]
fn quantize_q5_0() -> Result<()> {
    use k_quants::BlockQ5_0;

    let src = (0..32 * 4).map(|v| v as f32).collect::<Vec<_>>();
    let mut dst = vec![0f32; 32 * 4];
    let mut quant = vec![BlockQ5_0::zeros(); 4];
    BlockQ5_0::from_float(&src, &mut quant)?;
    BlockQ5_0::to_float(&quant, dst.as_mut_slice())?;
    assert_eq!(
        round_vector(&dst),
        &[
            -0.0, 1.938, 1.938, 3.875, 3.875, 5.813, 5.813, 7.75, 7.75, 9.688, 9.688, 11.625,
            11.625, 13.563, 13.563, 15.5, 15.5, 17.438, 17.438, 19.375, 19.375, 21.313, 21.313,
            23.25, 23.25, 25.188, 25.188, 27.125, 27.125, 29.063, 29.063, 31.0, 31.5, 31.5, 35.438,
            35.438, 35.438, 35.438, 39.375, 39.375, 39.375, 39.375, 43.313, 43.313, 43.313, 43.313,
            47.25, 47.25, 47.25, 47.25, 51.188, 51.188, 51.188, 51.188, 55.125, 55.125, 55.125,
            55.125, 59.063, 59.063, 59.063, 59.063, 63.0, 63.0, 65.313, 65.313, 65.313, 65.313,
            65.313, 71.25, 71.25, 71.25, 71.25, 71.25, 71.25, 77.188, 77.188, 77.188, 77.188,
            77.188, 77.188, 83.125, 83.125, 83.125, 83.125, 83.125, 83.125, 89.063, 89.063, 89.063,
            89.063, 89.063, 89.063, 95.0, 95.0, 95.0, 95.25, 95.25, 95.25, 95.25, 103.188, 103.188,
            103.188, 103.188, 103.188, 103.188, 103.188, 103.188, 111.125, 111.125, 111.125,
            111.125, 111.125, 111.125, 111.125, 111.125, 119.063, 119.063, 119.063, 119.063,
            119.063, 119.063, 119.063, 119.063, 127.0, 127.0, 127.0, 127.0
        ]
    );
    ggml_quantization_error_test::<BlockQ5_0>(GGML_MAX_QUANTIZATION_TOTAL_ERROR)?;
    Ok(())
}

#[test]
fn quantize_q5_1() -> Result<()> {
    use k_quants::BlockQ5_1;

    let src = (0..32 * 4).map(|v| v as f32).collect::<Vec<_>>();
    let mut dst = vec![0f32; 32 * 4];
    let mut quant = vec![BlockQ5_1::zeros(); 4];
    BlockQ5_1::from_float(&src, &mut quant)?;
    BlockQ5_1::to_float(&quant, dst.as_mut_slice())?;
    assert_eq!(
        dst,
        &[
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
            16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0,
            30.0, 31.0, 32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0,
            44.0, 45.0, 46.0, 47.0, 48.0, 49.0, 50.0, 51.0, 52.0, 53.0, 54.0, 55.0, 56.0, 57.0,
            58.0, 59.0, 60.0, 61.0, 62.0, 63.0, 64.0, 65.0, 66.0, 67.0, 68.0, 69.0, 70.0, 71.0,
            72.0, 73.0, 74.0, 75.0, 76.0, 77.0, 78.0, 79.0, 80.0, 81.0, 82.0, 83.0, 84.0, 85.0,
            86.0, 87.0, 88.0, 89.0, 90.0, 91.0, 92.0, 93.0, 94.0, 95.0, 96.0, 97.0, 98.0, 99.0,
            100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0, 108.0, 109.0, 110.0, 111.0,
            112.0, 113.0, 114.0, 115.0, 116.0, 117.0, 118.0, 119.0, 120.0, 121.0, 122.0, 123.0,
            124.0, 125.0, 126.0, 127.0
        ]
    );

    ggml_quantization_error_test::<BlockQ5_1>(GGML_MAX_QUANTIZATION_TOTAL_ERROR)?;
    Ok(())
}

/// Generates a small test vector ranging from -`bound` to `bound` with `size` steps
fn get_test_vector(bound: f32, size: usize) -> (Vec<f32>, Vec<f32>) {
    assert!(
        size % crate::quantized::k_quants::QK_K == 0,
        "size must be a multiple of {}",
        crate::quantized::k_quants::QK_K
    );

    let src = (0..size)
        .map(|v| (v as f32 - size as f32 / 2.) * bound / (size as f32 / 2.))
        .collect::<Vec<_>>();

    let dst = vec![0f32; size];
    assert_eq!([src[0], src[size / 2]], [-bound, 0.0]);
    (src, dst)
}

/// Round a vector
fn round_vector(values: &[f32]) -> Vec<f32> {
    values
        .iter()
        .map(|x| (1000. * x).round() / 1000.)
        .collect::<Vec<_>>()
}

fn compare_with_error(values: &[f32], expected: &[f32], tolerance: f32) {
    for (i, (value, expected_value)) in values.iter().zip(expected.iter()).enumerate() {
        let difference = (value - expected_value).abs();

        assert!(
            difference < tolerance,
            "Error at index {}: value = {}, expected = {}. Difference = {} exceeds tolerance = {}.",
            i,
            value,
            expected_value,
            difference,
            tolerance
        );
    }
}

/// Creates a vector simillarly to the one used in GGML unit tests: https://github.com/ggerganov/llama.cpp/blob/master/tests/test-quantize-fns.cpp#L26-L30
fn create_ggml_like_vector(offset: f32) -> Vec<f32> {
    (0..GGML_TEST_SIZE)
        .map(|i| 0.1 + 2.0 * (i as f32 + offset).cos())
        .collect()
}

/// Calculates the root mean square error between two vectors
fn calculate_rmse(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let sum = a
        .iter()
        .zip(b)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        .sqrt();
    sum / a.len() as f32
}

/// Mirrores the GGML quanitzation unit test: https://github.com/ggerganov/llama.cpp/blob/master/tests/test-quantize-fns.cpp#L43-L50
fn ggml_quantization_error_test<T: GgmlType>(max_error: f32) -> Result<()> {
    let src = create_ggml_like_vector(0.0);
    let mut dst = vec![0.0; GGML_TEST_SIZE];
    let _quant = quantize_roundtrip::<T>(src.as_slice(), dst.as_mut_slice())?;
    let error = calculate_rmse(src.as_slice(), dst.as_slice());
    if error > max_error {
        candle_core::bail!(
            "Quantization error {} exceeds max error {}",
            error,
            max_error
        );
    }
    Ok(())
}

fn quantize_roundtrip<T: GgmlType>(src: &[f32], dst: &mut [f32]) -> Result<Vec<T>> {
    let mut quant = vec![T::zeros(); src.len() / T::BLCK_SIZE];
    T::from_float(src, &mut quant)?;
    T::to_float(&quant, dst)?;
    Ok(quant)
}

#[test]
fn quantize_q2k() -> Result<()> {
    use k_quants::BlockQ2K;

    let (src, mut dst) = get_test_vector(0.5, 1024);
    let _quant = quantize_roundtrip::<BlockQ2K>(src.as_slice(), dst.as_mut_slice())?;
    compare_with_error(dst.as_slice(), src.as_slice(), 0.1);

    // Test some specific values
    assert_eq!(
        [src[0], src[128], src[256], src[512], src[800], src[1023]],
        [-0.5, -0.375, -0.25, 0.0, 0.28125, 0.49902344]
    );
    let dst = round_vector(&dst);
    assert_eq!(
        [dst[0], dst[128], dst[256], dst[512], dst[800], dst[1023]],
        [-0.499, -0.366, -0.249, 0.0, 0.295, 0.492]
    );

    let (src_big, mut dst_big) = get_test_vector(128.0, 1024);
    let _quant_big = quantize_roundtrip::<BlockQ2K>(src_big.as_slice(), dst_big.as_mut_slice())?;
    compare_with_error(dst_big.as_slice(), src_big.as_slice(), 6.0);

    ggml_quantization_error_test::<BlockQ2K>(GGML_MAX_QUANTIZATION_TOTAL_ERROR_2BITS)?;
    Ok(())
}

#[test]
fn quantize_q3k() -> Result<()> {
    use k_quants::BlockQ3K;

    let (src, mut dst) = get_test_vector(0.5, 1024);
    let _quant = quantize_roundtrip::<BlockQ3K>(src.as_slice(), dst.as_mut_slice())?;
    compare_with_error(dst.as_slice(), src.as_slice(), 0.03);

    // Test some specific values
    assert_eq!(
        [src[0], src[128], src[256], src[512], src[800], src[1023]],
        [-0.5, -0.375, -0.25, 0.0, 0.28125, 0.49902344]
    );
    let dst = round_vector(&dst);
    assert_eq!(
        [dst[0], dst[128], dst[256], dst[512], dst[800], dst[1023]],
        [-0.493, -0.37, -0.243, -0.0, 0.292, 0.492]
    );

    let (src_big, mut dst_big) = get_test_vector(128.0, 1024);
    let _quant_big = quantize_roundtrip::<BlockQ3K>(src_big.as_slice(), dst_big.as_mut_slice())?;
    compare_with_error(dst_big.as_slice(), src_big.as_slice(), 3.5);

    ggml_quantization_error_test::<BlockQ3K>(GGML_MAX_QUANTIZATION_TOTAL_ERROR_3BITS)?;
    Ok(())
}

#[test]
fn quantize_q4k() -> Result<()> {
    use k_quants::BlockQ4K;

    let (src, mut dst) = get_test_vector(0.5, 1024);
    let _quant = quantize_roundtrip::<BlockQ4K>(src.as_slice(), dst.as_mut_slice())?;
    compare_with_error(dst.as_slice(), src.as_slice(), 0.017);

    // Test some specific values
    assert_eq!(
        [src[0], src[128], src[256], src[512], src[800], src[1023]],
        [-0.5, -0.375, -0.25, 0.0, 0.28125, 0.49902344]
    );
    let dst = round_vector(&dst);
    assert_eq!(
        [dst[0], dst[128], dst[256], dst[512], dst[800], dst[1023]],
        [-0.5, -0.373, -0.25, 0.0, 0.288, 0.498]
    );

    let (src_big, mut dst_big) = get_test_vector(128.0, 1024);
    let _quant_big = quantize_roundtrip::<BlockQ4K>(src_big.as_slice(), dst_big.as_mut_slice())?;
    compare_with_error(dst_big.as_slice(), src_big.as_slice(), 4.5);

    ggml_quantization_error_test::<BlockQ4K>(GGML_MAX_QUANTIZATION_TOTAL_ERROR)?;
    Ok(())
}

#[test]
fn quantize_q5k() -> Result<()> {
    use k_quants::BlockQ5K;

    let (src, mut dst) = get_test_vector(0.5, 1024);
    let _quant = quantize_roundtrip::<BlockQ5K>(src.as_slice(), dst.as_mut_slice())?;
    compare_with_error(dst.as_slice(), src.as_slice(), 0.008);

    // Test some specific values
    assert_eq!(
        [src[0], src[128], src[256], src[512], src[800], src[1023]],
        [-0.5, -0.375, -0.25, 0.0, 0.28125, 0.49902344]
    );
    let dst = round_vector(&dst);
    assert_eq!(
        [dst[0], dst[128], dst[256], dst[512], dst[800], dst[1023]],
        [-0.499, -0.372, -0.249, 0.001, 0.279, 0.499]
    );

    let (src_big, mut dst_big) = get_test_vector(128.0, 1024);
    let _quant_big = quantize_roundtrip::<BlockQ5K>(src_big.as_slice(), dst_big.as_mut_slice())?;
    compare_with_error(dst_big.as_slice(), src_big.as_slice(), 2.5);

    ggml_quantization_error_test::<BlockQ5K>(GGML_MAX_QUANTIZATION_TOTAL_ERROR)?;

    Ok(())
}

#[test]
fn quantize_q6k() -> Result<()> {
    use k_quants::BlockQ6K;

    let (src, mut dst) = get_test_vector(0.5, 1024);
    let _quant = quantize_roundtrip::<BlockQ6K>(src.as_slice(), dst.as_mut_slice())?;
    compare_with_error(dst.as_slice(), src.as_slice(), 0.008);

    // Test some specific values
    assert_eq!(
        [src[0], src[128], src[256], src[512], src[800], src[1023]],
        [-0.5, -0.375, -0.25, 0.0, 0.28125, 0.49902344]
    );
    let dst = round_vector(&dst);
    assert_eq!(
        [dst[0], dst[128], dst[256], dst[512], dst[800], dst[1023]],
        [-0.497, -0.372, -0.25, -0.0, 0.284, 0.5]
    );

    let (src_big, mut dst_big) = get_test_vector(128.0, 1024);
    let _quant_big = quantize_roundtrip::<BlockQ6K>(src_big.as_slice(), dst_big.as_mut_slice())?;
    compare_with_error(dst_big.as_slice(), src_big.as_slice(), 2.0);

    ggml_quantization_error_test::<BlockQ6K>(GGML_MAX_QUANTIZATION_TOTAL_ERROR)?;

    Ok(())
}

#[test]
fn quantize_q8k() -> Result<()> {
    use k_quants::BlockQ8K;

    let (src, mut dst) = get_test_vector(0.5, 1024);
    let _quant = quantize_roundtrip::<BlockQ8K>(src.as_slice(), dst.as_mut_slice())?;
    compare_with_error(dst.as_slice(), src.as_slice(), 0.003);

    // Test some specific values
    assert_eq!(
        [src[0], src[128], src[256], src[512], src[800], src[1023]],
        [-0.5, -0.375, -0.25, 0.0, 0.28125, 0.49902344]
    );
    let dst = round_vector(&dst);
    assert_eq!(
        [dst[0], dst[128], dst[256], dst[512], dst[800], dst[1023]],
        [-0.5, -0.375, -0.25, -0.0, 0.281, 0.499]
    );

    let (src_big, mut dst_big) = get_test_vector(128.0, 1024);
    let _quant_big = quantize_roundtrip::<BlockQ8K>(src_big.as_slice(), dst_big.as_mut_slice())?;
    compare_with_error(dst_big.as_slice(), src_big.as_slice(), 0.6);

    ggml_quantization_error_test::<BlockQ8K>(GGML_MAX_QUANTIZATION_TOTAL_ERROR)?;

    Ok(())
}

/// Very simple dot product implementation
fn vec_dot_reference(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

/// Returns the error achieved by the GGML matmul unit test.
fn ggml_reference_matmul_error(dtype: GgmlDType) -> Result<f32> {
    let err = match dtype {
        GgmlDType::F16 => 0.000010,
        GgmlDType::Q2K => 0.004086,
        GgmlDType::Q3K => 0.016148,
        GgmlDType::Q4K => 0.002425,
        GgmlDType::Q5K => 0.000740,
        GgmlDType::Q6K => 0.000952,
        GgmlDType::Q4_0 => 0.001143,
        GgmlDType::Q4_1 => 0.007784,
        GgmlDType::Q5_0 => 0.001353,
        GgmlDType::Q5_1 => 0.001363,
        GgmlDType::Q8_0 => 0.000092,
        _ => candle_core::bail!("No GGML results for quantization type {dtype:?}",),
    };
    Ok(err)
}

/// Mirrores the GGML matmul unit test: https://github.com/ggerganov/llama.cpp/blob/master/tests/test-quantize-fns.cpp#L76-L91
fn ggml_matmul_error_test<T: GgmlType>() -> Result<()> {
    let a = create_ggml_like_vector(0.0);
    let b = create_ggml_like_vector(1.0);
    let length = a.len();

    let mut a_quant = vec![T::zeros(); length / T::BLCK_SIZE];
    let mut b_quant = vec![T::VecDotType::zeros(); length / T::VecDotType::BLCK_SIZE];
    T::from_float(&a, &mut a_quant)?;
    T::VecDotType::from_float(&b, &mut b_quant)?;

    let result = T::vec_dot(length, &a_quant, &b_quant)?;
    let reference_result = vec_dot_reference(&a, &b);

    let error = (result - reference_result).abs() / length as f32;

    let ggml_error = ggml_reference_matmul_error(T::DTYPE)?;

    if !error.is_finite() || error > GGML_MAX_DOT_PRODUCT_ERROR {
        candle_core::bail!(
            "Dot product error {} exceeds max error {}",
            error,
            GGML_MAX_DOT_PRODUCT_ERROR
        );
    }

    // We diverge slightly due to different rounding behavior / f16 to f32 conversions in GGML
    // => we use a slightly higher error threshold
    const ERROR_LENIENCY: f32 = 0.00001;
    if error - ERROR_LENIENCY > ggml_error {
        candle_core::bail!(
            "Dot product error {} exceeds ggml reference error {}",
            error,
            ggml_error
        );
    }
    Ok(())
}

/// generates random tensors of size `m x k` and `n x k` and calculates their expected matrix multiplication result.
fn get_random_tensors(
    m: usize,
    k: usize,
    n: usize,
    device: &Device,
) -> Result<(Tensor, Tensor, Tensor)> {
    let mut rng = StdRng::seed_from_u64(314159265358979);

    let lhs = (0..m * k)
        .map(|_| rng.gen::<f32>() - 0.5)
        .collect::<Vec<_>>();
    let rhs = (0..n * k)
        .map(|_| rng.gen::<f32>() - 0.5)
        .collect::<Vec<_>>();

    let lhs = Tensor::from_vec(lhs, (m, k), device)?;
    let rhs = Tensor::from_vec(rhs, (n, k), device)?;

    let mm = lhs.matmul(&rhs.t()?)?;
    Ok((lhs, rhs, mm))
}

#[test]
fn quantized_matmul_q2k() -> Result<()> {
    use k_quants::BlockQ2K;

    let cpu = &Device::Cpu;
    let (m, k, n) = (11, 512, 21);
    let (lhs, rhs, mm) = get_random_tensors(m, k, n, cpu)?;
    assert_eq!(mm.dims(), [m, n]);
    let dst = mm.flatten_all()?.to_vec1::<f32>()?;
    let dst = round_vector(&[dst[0], dst[m * n / 3], dst[m * n * 2 / 3], dst[m * n - 1]]);
    assert_eq!(dst, [1.262, 1.513, -0.208, 1.702]);

    let rhs = quantized::QTensor::quantize::<BlockQ2K>(&rhs)?;
    let rhs = quantized::QMatMul::from_qtensor(rhs);
    let mm = rhs.forward(&lhs)?;

    assert_eq!(mm.dims(), [m, n]);
    let dst = mm.flatten_all()?.to_vec1::<f32>()?;
    let dst = round_vector(&[dst[0], dst[m * n / 3], dst[m * n * 2 / 3], dst[m * n - 1]]);
    assert_eq!(dst, [0.916, 0.422, 0.215, 1.668]);

    ggml_matmul_error_test::<BlockQ2K>()?;

    Ok(())
}

#[test]
fn quantized_matmul_q3k() -> Result<()> {
    use k_quants::BlockQ3K;

    let cpu = &Device::Cpu;
    let (m, k, n) = (11, 512, 21);
    let (lhs, rhs, mm) = get_random_tensors(m, k, n, cpu)?;
    assert_eq!(mm.dims(), [m, n]);
    let dst = mm.flatten_all()?.to_vec1::<f32>()?;
    let dst = round_vector(&[dst[0], dst[m * n / 3], dst[m * n * 2 / 3], dst[m * n - 1]]);
    assert_eq!(dst, [1.262, 1.513, -0.208, 1.702]);

    let rhs = quantized::QTensor::quantize::<BlockQ3K>(&rhs)?;
    let rhs = quantized::QMatMul::from_qtensor(rhs);
    let mm = rhs.forward(&lhs)?;

    assert_eq!(mm.dims(), [m, n]);
    let dst = mm.flatten_all()?.to_vec1::<f32>()?;
    let dst = round_vector(&[dst[0], dst[m * n / 3], dst[m * n * 2 / 3], dst[m * n - 1]]);
    assert_eq!(dst, [1.029, 1.418, -0.314, 1.495]);

    ggml_matmul_error_test::<BlockQ3K>()?;

    Ok(())
}

#[test]
fn quantized_matmul_q4k() -> Result<()> {
    use k_quants::BlockQ4K;

    let cpu = &Device::Cpu;
    let (m, k, n) = (11, 512, 21);
    let (lhs, rhs, mm) = get_random_tensors(m, k, n, cpu)?;
    assert_eq!(mm.dims(), [m, n]);
    let dst = mm.flatten_all()?.to_vec1::<f32>()?;
    let dst = round_vector(&[dst[0], dst[m * n / 3], dst[m * n * 2 / 3], dst[m * n - 1]]);
    assert_eq!(dst, [1.262, 1.513, -0.208, 1.702]);

    let rhs = quantized::QTensor::quantize::<BlockQ4K>(&rhs)?;
    let rhs = quantized::QMatMul::from_qtensor(rhs);
    let mm = rhs.forward(&lhs)?;

    assert_eq!(mm.dims(), [m, n]);
    let dst = mm.flatten_all()?.to_vec1::<f32>()?;
    let dst = round_vector(&[dst[0], dst[m * n / 3], dst[m * n * 2 / 3], dst[m * n - 1]]);
    assert_eq!(dst, [1.125, 1.435, -0.201, 1.589]);

    ggml_matmul_error_test::<BlockQ4K>()?;

    Ok(())
}

#[test]
fn quantized_matmul_q5k() -> Result<()> {
    use k_quants::BlockQ5K;

    let cpu = &Device::Cpu;
    let (m, k, n) = (11, 512, 21);
    let (lhs, rhs, mm) = get_random_tensors(m, k, n, cpu)?;
    assert_eq!(mm.dims(), [m, n]);
    let dst = mm.flatten_all()?.to_vec1::<f32>()?;
    let dst = round_vector(&[dst[0], dst[m * n / 3], dst[m * n * 2 / 3], dst[m * n - 1]]);
    assert_eq!(dst, [1.262, 1.513, -0.208, 1.702]);

    let rhs = quantized::QTensor::quantize::<BlockQ5K>(&rhs)?;
    let rhs = quantized::QMatMul::from_qtensor(rhs);
    let mm = rhs.forward(&lhs)?;

    assert_eq!(mm.dims(), [m, n]);
    let dst = mm.flatten_all()?.to_vec1::<f32>()?;
    let dst = round_vector(&[dst[0], dst[m * n / 3], dst[m * n * 2 / 3], dst[m * n - 1]]);
    assert_eq!(dst, [1.192, 1.491, -0.18, 1.743]);

    //Expected: 0.000740408897
    ggml_matmul_error_test::<BlockQ5K>()?;

    Ok(())
}

#[test]
fn quantized_matmul_q6k() -> Result<()> {
    use k_quants::BlockQ6K;

    let cpu = &Device::Cpu;
    let (m, k, n) = (11, 512, 21);
    let (lhs, rhs, mm) = get_random_tensors(m, k, n, cpu)?;
    assert_eq!(mm.dims(), [m, n]);
    let dst = mm.flatten_all()?.to_vec1::<f32>()?;
    let dst = round_vector(&[dst[0], dst[m * n / 3], dst[m * n * 2 / 3], dst[m * n - 1]]);
    assert_eq!(dst, [1.262, 1.513, -0.208, 1.702]);

    let rhs = quantized::QTensor::quantize::<BlockQ6K>(&rhs)?;
    let rhs = quantized::QMatMul::from_qtensor(rhs);
    let mm = rhs.forward(&lhs)?;

    assert_eq!(mm.dims(), [m, n]);
    let dst = mm.flatten_all()?.to_vec1::<f32>()?;
    let dst = round_vector(&[dst[0], dst[m * n / 3], dst[m * n * 2 / 3], dst[m * n - 1]]);
    assert_eq!(dst, [1.324, 1.49, -0.164, 1.741]);

    ggml_matmul_error_test::<BlockQ6K>()?;
    Ok(())
}
