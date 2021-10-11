/// Used to assert at compile time that two types have the same `size_of`.
///
/// Used as trait bound for `slice_cast` and `slice_cast_mut` macros.
pub struct SizeOf<const V1: usize, const V2: usize> {}

/// Used to assert at compile time that two types have the same `align_of`.
///
/// Used as trait bound for `slice_cast` and `slice_cast_mut` macros.
pub struct AlignOf<const V1: usize, const V2: usize> {}

/// Used by `SizeOf` and `AlignOf` to assert that their values are the same.
pub trait IsEqual {}

impl<const V: usize> IsEqual for SizeOf<V, V> {}
impl<const V: usize> IsEqual for AlignOf<V, V> {}

/// Safely converts a `&[T]` into `&[U]` with compile time safety guards.
///
/// # Usage
///
/// ```
/// # use s3sat_utils::slice_cast;
/// let given = [1i32, 2, 3, 4, 5];
/// let expected = [1u32, 2, 3, 4, 5];
/// let actual = slice_cast!(<i32, u32>(&given));
/// assert_eq!(actual, &expected);
/// ```
///
/// The below code snippet fails to compile since `i32` and `i64`
/// do not have the same `size_of` and `align_of`.
///
/// ```compile_fail
/// # use s3sat_utils::slice_cast;
/// let given = [1i32, 2, 3, 4, 5];
/// let actual = slice_cast!(<i32, i64>(&given));
/// ```
#[macro_export]
macro_rules! slice_cast {
    ( <$from_type:ty, $into_type:ty> ( $slice:expr ) ) => {{
        fn slice_cast(from_slice: &[$from_type]) -> &[$into_type]
        where
            $crate::slice_cast::SizeOf<
                { ::core::mem::size_of::<$from_type>() },
                { ::core::mem::size_of::<$into_type>() },
            >: $crate::slice_cast::IsEqual,
            $crate::slice_cast::AlignOf<
                { ::core::mem::align_of::<$from_type>() },
                { ::core::mem::align_of::<$into_type>() },
            >: $crate::slice_cast::IsEqual,
        {
            // Safety: This unsafe operation is safe due to the additional
            //         trait bounds above that assert at compilation time
            //         that the slice's element types have the same `size_of`
            //         and `align_of`.
            unsafe {
                ::core::slice::from_raw_parts(
                    from_slice.as_ptr() as *const $into_type,
                    from_slice.len(),
                )
            }
        }
        slice_cast($slice)
    }};
}

/// Safely converts a `&mut [T]` into `&mut [U]` with compile time safety guards.
///
/// # Usage
///
/// ```
/// # use s3sat_utils::slice_cast_mut;
/// let mut given = [1i32, 2, 3, 4, 5];
/// let expected = [1u32, 2, 3, 4, 5];
/// let actual = slice_cast_mut!(<i32, u32>(&mut given));
/// assert_eq!(actual, &expected);
/// ```
///
/// The below code snippet fails to compile since `i32` and `i64`
/// do not have the same `size_of` and `align_of`.
///
/// ```compile_fail
/// # use s3sat_utils::slice_cast_mut;
/// let mut given = [1i32, 2, 3, 4, 5];
/// let actual = slice_cast_mut!(<i32, i64>(&given));
/// ```
#[macro_export]
macro_rules! slice_cast_mut {
    ( <$from_type:ty, $into_type:ty> ( $slice:expr ) ) => {{
        fn slice_cast_mut(from_slice: &mut [$from_type]) -> &mut [$into_type]
        where
            $crate::slice_cast::SizeOf<
                { ::core::mem::size_of::<$from_type>() },
                { ::core::mem::size_of::<$into_type>() },
            >: $crate::slice_cast::IsEqual,
            $crate::slice_cast::AlignOf<
                { ::core::mem::align_of::<$from_type>() },
                { ::core::mem::align_of::<$into_type>() },
            >: $crate::slice_cast::IsEqual,
        {
            // Safety: This unsafe operation is safe due to the additional
            //         trait bounds above that assert at compilation time
            //         that the slice's element types have the same `size_of`
            //         and `align_of`.
            unsafe {
                ::core::slice::from_raw_parts_mut(
                    from_slice.as_ptr() as *mut $into_type,
                    from_slice.len(),
                )
            }
        }
        slice_cast_mut($slice)
    }};
}

#[test]
fn slice_cast_works() {
    #[derive(Debug, PartialEq)]
    #[repr(transparent)]
    pub struct ReprU32(u32);

    let us = [-1i32, -2, -3, -4, -5];

    let vs = slice_cast!(<i32, u32>(&us));
    assert_eq!(vs, us.map(|v| v as u32).as_ref());

    let vs = slice_cast!(<u32, ReprU32>(vs));
    assert_eq!(vs, us.map(|v| v as u32).map(ReprU32).as_ref());
}

#[test]
fn slice_cast_mut_works() {
    #[derive(Debug, PartialEq)]
    #[repr(transparent)]
    pub struct ReprU32(u32);

    const VALUES: [i32; 5] = [-1i32, -2, -3, -4, -5];

    let mut values = VALUES;

    let converted = slice_cast_mut!(<i32, u32>(&mut values));
    assert_eq!(converted, VALUES.map(|v| v as u32).as_ref());

    let repr32s = slice_cast_mut!(<u32, ReprU32>(converted));
    assert_eq!(repr32s, VALUES.map(|v| v as u32).map(ReprU32).as_ref());
}
