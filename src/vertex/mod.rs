/*!
Contains everything related to vertex sources.

When you draw, you need to pass one or several sources of vertex attributes. This is done with
the first parameter to the `draw` function.

## Vertex

The main trait of this module is `Vertex`, which must be implemented on structs whose instances
describe individual vertices. The trait is unsafe to implement, so you are encouraged to use the
`implement_vertex!` macro instead:

```
# #[macro_use]
# extern crate glium;
# extern crate glutin;
# fn main() {
#[derive(Copy, Clone)]
struct MyVertex {
    position: [f32; 3],
    texcoords: [f32; 2],
}

// you must pass the list of members to the macro
implement_vertex!(MyVertex, position, texcoords);
# }
```

## Vertex buffer

Once you have a struct that implements the `Vertex` trait, you can build an array of vertices and
upload it to the video memory by creating a `VertexBuffer`.

```no_run
# let display: glium::Display = unsafe { ::std::mem::MaybeUninit::uninit().assume_init() };
# #[derive(Copy, Clone)]
# struct MyVertex {
#     position: [f32; 3],
#     texcoords: [f32; 2],
# }
# impl glium::vertex::Vertex for MyVertex {
#     fn build_bindings() -> glium::vertex::VertexFormat { unimplemented!() }
# }
let data = &[
    MyVertex {
        position: [0.0, 0.0, 0.4],
        texcoords: [0.0, 1.0]
    },
    MyVertex {
        position: [12.0, 4.5, -1.8],
        texcoords: [1.0, 0.5]
    },
    MyVertex {
        position: [-7.124, 0.1, 0.0],
        texcoords: [0.0, 0.4]
    },
];

let vertex_buffer = glium::vertex::VertexBuffer::new(&display, data);
```

## Drawing

When you draw, you can pass either a single vertex source or a tuple of multiple sources.
Each source can be:

 - A reference to a `VertexBuffer`.
 - A slice of a vertex buffer, by calling `vertex_buffer.slice(start .. end).unwrap()`.
 - A vertex buffer where each element corresponds to an instance, by
   calling `vertex_buffer.per_instance()`.
 - The same with a slice, by calling `vertex_buffer.slice(start .. end).unwrap().per_instance()`.
 - A marker indicating a number of vertex sources, with `glium::vertex::EmptyVertexAttributes`.
 - A marker indicating a number of instances, with `glium::vertex::EmptyInstanceAttributes`.

```no_run
# use glium::Surface;
# let display: glium::Display = unsafe { ::std::mem::MaybeUninit::uninit().assume_init() };
# #[derive(Copy, Clone)]
# struct MyVertex { position: [f32; 3], texcoords: [f32; 2], }
# impl glium::vertex::Vertex for MyVertex {
#     fn build_bindings() -> glium::vertex::VertexFormat { unimplemented!() }
# }
# let program: glium::program::Program = unsafe { ::std::mem::MaybeUninit::uninit().assume_init() };
# let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
# let uniforms = glium::uniforms::EmptyUniforms;
# let vertex_buffer: glium::vertex::VertexBuffer<MyVertex> = unsafe { ::std::mem::MaybeUninit::uninit().assume_init() };
# let vertex_buffer2: glium::vertex::VertexBuffer<MyVertex> = unsafe { ::std::mem::MaybeUninit::uninit().assume_init() };
# let mut frame = display.draw();
// drawing with a single vertex buffer
frame.draw(&vertex_buffer, &indices, &program, &uniforms, &Default::default()).unwrap();

// drawing with two parallel vertex buffers
frame.draw((&vertex_buffer, &vertex_buffer2), &indices, &program,
           &uniforms, &Default::default()).unwrap();

// drawing without a vertex source
frame.draw(glium::vertex::EmptyVertexAttributes { len: 12 }, &indices, &program,
           &uniforms, &Default::default()).unwrap();

// drawing a slice of a vertex buffer
frame.draw(vertex_buffer.slice(6 .. 24).unwrap(), &indices, &program,
           &uniforms, &Default::default()).unwrap();

// drawing slices of two vertex buffers
frame.draw((vertex_buffer.slice(6 .. 24).unwrap(), vertex_buffer2.slice(128 .. 146).unwrap()),
           &indices, &program, &uniforms, &Default::default()).unwrap();

// treating `vertex_buffer2` as a source of attributes per-instance instead of per-vertex
frame.draw((&vertex_buffer, vertex_buffer2.per_instance().unwrap()), &indices,
           &program, &uniforms, &Default::default()).unwrap();

// instancing without any per-instance attribute
frame.draw((&vertex_buffer, glium::vertex::EmptyInstanceAttributes { len: 36 }), &indices,
           &program, &uniforms, &Default::default()).unwrap();
```

Note that if you use `index::EmptyIndices` as indices the length of all vertex sources must
be the same, or a `DrawError::VerticesSourcesLengthMismatch` will be produced.

In all situation, the length of all per-instance sources must match, or
`DrawError::InstancesCountMismatch` will be returned.

# Transform feedback

Transform feedback allows you to write in a buffer the list of primitives that are generated by
the GPU.

To use it, you must first create a `TransformFeedbackSession` with
`TransformFeedbackSession::new()`. This function requires you to pass a buffer of the correct
type and a program. Then you must pass the `&TransformFeedbackSession` to the draw parameters.
The program you use when drawing must be the same as you the one you created the session
with, or else you will get an error.

*/
use std::iter::Chain;
use std::option::IntoIter;

pub use self::buffer::{VertexBuffer, VertexBufferAny};
pub use self::buffer::VertexBufferSlice;
pub use self::buffer::CreationError as BufferCreationError;
pub use self::format::{AttributeType, VertexFormat};
pub use self::transform_feedback::{is_transform_feedback_supported, TransformFeedbackSession};

use crate::buffer::BufferAnySlice;
use crate::CapabilitiesSource;

mod buffer;
mod format;
mod transform_feedback;

/// Describes the source to use for the vertices when drawing.
#[derive(Clone)]
pub enum VerticesSource<'a> {
    /// A buffer uploaded in the video memory.
    ///
    /// The second parameter is the number of vertices in the buffer.
    ///
    /// The third parameter tells whether or not this buffer is "per instance" (true) or
    /// "per vertex" (false).
    VertexBuffer(BufferAnySlice<'a>, &'a VertexFormat, bool),

    /// A marker indicating a "phantom list of attributes".
    Marker {
        /// Number of attributes.
        len: usize,

        /// Whether or not this buffer is "per instance" (true) or "per vertex" (false).
        per_instance: bool,
    },
}

/// Marker that can be passed instead of a buffer to indicate an empty list of buffers.
pub struct EmptyVertexAttributes {
    /// Number of phantom vertices.
    pub len: usize,
}

impl<'a> Into<VerticesSource<'a>> for EmptyVertexAttributes {
    #[inline]
    fn into(self) -> VerticesSource<'a> {
        VerticesSource::Marker { len: self.len, per_instance: false }
    }
}

/// Marker that can be passed instead of a buffer to indicate an empty list of buffers.
pub struct EmptyInstanceAttributes {
    /// Number of phantom vertices.
    pub len: usize,
}

impl<'a> Into<VerticesSource<'a>> for EmptyInstanceAttributes {
    #[inline]
    fn into(self) -> VerticesSource<'a> {
        VerticesSource::Marker { len: self.len, per_instance: true }
    }
}

/// Marker that instructs glium that the buffer is to be used per instance.
pub struct PerInstance<'a>(BufferAnySlice<'a>, &'a VertexFormat);

impl<'a> Into<VerticesSource<'a>> for PerInstance<'a> {
    #[inline]
    fn into(self) -> VerticesSource<'a> {
        VerticesSource::VertexBuffer(self.0, self.1, true)
    }
}

/// Objects that describe multiple vertex sources.
pub trait MultiVerticesSource<'a> {
    /// Iterator that enumerates each source.
    type Iterator: Iterator<Item = VerticesSource<'a>>;

    /// Iterates over the `VerticesSource`.
    fn iter(self) -> Self::Iterator;
}

impl<'a, T> MultiVerticesSource<'a> for T
    where T: Into<VerticesSource<'a>>
{
    type Iterator = IntoIter<VerticesSource<'a>>;

    #[inline]
    fn iter(self) -> IntoIter<VerticesSource<'a>> {
        Some(self.into()).into_iter()
    }
}

macro_rules! impl_for_tuple {
    ($t:ident) => (
        impl<'a, $t> MultiVerticesSource<'a> for ($t,)
            where $t: Into<VerticesSource<'a>>
        {
            type Iterator = IntoIter<VerticesSource<'a>>;

            #[inline]
            fn iter(self) -> IntoIter<VerticesSource<'a>> {
                Some(self.0.into()).into_iter()
            }
        }
    );

    ($t1:ident, $t2:ident) => (
        #[allow(non_snake_case)]
        impl<'a, $t1, $t2> MultiVerticesSource<'a> for ($t1, $t2)
            where $t1: Into<VerticesSource<'a>>, $t2: Into<VerticesSource<'a>>
        {
            type Iterator = Chain<<($t1,) as MultiVerticesSource<'a>>::Iterator,
                                  <($t2,) as MultiVerticesSource<'a>>::Iterator>;

            #[inline]
            fn iter(self) -> Chain<<($t1,) as MultiVerticesSource<'a>>::Iterator,
                                   <($t2,) as MultiVerticesSource<'a>>::Iterator>
            {
                let ($t1, $t2) = self;
                Some($t1.into()).into_iter().chain(($t2,).iter())
            }
        }

        impl_for_tuple!($t2);
    );

    ($t1:ident, $($t2:ident),+) => (
        #[allow(non_snake_case)]
        impl<'a, $t1, $($t2),+> MultiVerticesSource<'a> for ($t1, $($t2),+)
            where $t1: Into<VerticesSource<'a>>, $($t2: Into<VerticesSource<'a>>),+
        {
            type Iterator = Chain<<($t1,) as MultiVerticesSource<'a>>::Iterator,
                                  <($($t2),+) as MultiVerticesSource<'a>>::Iterator>;

            #[inline]
            fn iter(self) -> Chain<<($t1,) as MultiVerticesSource<'a>>::Iterator,
                                  <($($t2),+) as MultiVerticesSource<'a>>::Iterator>
            {
                let ($t1, $($t2),+) = self;
                Some($t1.into()).into_iter().chain(($($t2),+).iter())
            }
        }

        impl_for_tuple!($($t2),+);
    );
}

impl_for_tuple!(A, B, C, D, E, F, G);

/// Trait for structures that represent a vertex.
///
/// Instead of implementing this trait yourself, it is recommended to use the `implement_vertex!`
/// macro instead.
// TODO: this should be `unsafe`, but that would break the syntax extension
pub trait Vertex: Copy + Sized {
    /// Builds the `VertexFormat` representing the layout of this element.
    fn build_bindings() -> VertexFormat;

    /// Returns true if the backend supports this vertex format.
    fn is_supported<C: ?Sized>(caps: &C) -> bool where C: CapabilitiesSource {
        let format = Self::build_bindings();

        for &(_, _, ref ty, _) in format.iter() {
            if !ty.is_supported(caps) {
                return false;
            }
        }

        true
    }
}

/// Trait for types that can be used as vertex attributes.
pub unsafe trait Attribute: Sized {
    /// Get the type of data.
    fn get_type() -> AttributeType;

    /// Returns true if the backend supports this type of attribute.
    #[inline]
    fn is_supported<C: ?Sized>(caps: &C) -> bool where C: CapabilitiesSource {
        Self::get_type().is_supported(caps)
    }
}
