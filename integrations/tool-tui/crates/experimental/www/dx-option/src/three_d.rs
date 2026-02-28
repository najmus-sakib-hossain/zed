//! # 3D Environment and Models
//!
//! Binary-optimized 3D rendering using WebGL/WebGPU.
//! Pre-computed transforms and binary vertex data for zero-parse rendering.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3, Vec4};

// ============================================================================
// Binary Vertex Format (32 bytes per vertex)
// ============================================================================

/// Binary vertex format - optimized for GPU upload
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BinaryVertex {
    /// Position XYZ (12 bytes)
    pub position: [f32; 3],
    /// Normal XYZ (12 bytes)
    pub normal: [f32; 3],
    /// UV coordinates (8 bytes)
    pub uv: [f32; 2],
}

impl BinaryVertex {
    /// Create a new vertex
    #[inline]
    pub const fn new(pos: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position: pos,
            normal,
            uv,
        }
    }
}

// ============================================================================
// 3D Transform (64 bytes)
// ============================================================================

/// Binary 3D transform - directly uploadable to GPU
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BinaryTransform {
    /// Model matrix (64 bytes)
    pub matrix: [[f32; 4]; 4],
}

impl BinaryTransform {
    /// Create identity transform
    pub const fn identity() -> Self {
        Self {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Create from position, rotation, scale
    pub fn from_trs(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let mat = Mat4::from_scale_rotation_translation(scale, rotation, translation);
        Self {
            matrix: mat.to_cols_array_2d(),
        }
    }

    /// Create translation transform
    pub fn translation(x: f32, y: f32, z: f32) -> Self {
        Self::from_trs(Vec3::new(x, y, z), Quat::IDENTITY, Vec3::ONE)
    }

    /// Create rotation transform (euler angles in radians)
    pub fn rotation(pitch: f32, yaw: f32, roll: f32) -> Self {
        let rot = Quat::from_euler(glam::EulerRot::XYZ, pitch, yaw, roll);
        Self::from_trs(Vec3::ZERO, rot, Vec3::ONE)
    }

    /// Multiply transforms
    pub fn multiply(&self, other: &Self) -> Self {
        let a = Mat4::from_cols_array_2d(&self.matrix);
        let b = Mat4::from_cols_array_2d(&other.matrix);
        Self {
            matrix: (a * b).to_cols_array_2d(),
        }
    }
}

// ============================================================================
// Pre-built 3D Models (Binary Mesh Data)
// ============================================================================

/// DX Logo 3D model - the "D" and "X" letters extruded
pub struct DxLogoMesh;

impl DxLogoMesh {
    /// Generate vertices for DX logo
    pub fn generate_vertices() -> Vec<BinaryVertex> {
        let mut vertices = Vec::with_capacity(256);
        
        // Letter "D" - simplified as a curved shape
        // Front face
        let d_points = [
            (0.0, 0.0), (0.0, 1.0), (0.3, 1.0),
            (0.5, 0.8), (0.5, 0.2), (0.3, 0.0),
        ];
        
        for i in 0..d_points.len() {
            let (x, y) = d_points[i];
            let next = d_points[(i + 1) % d_points.len()];
            
            // Front triangle
            vertices.push(BinaryVertex::new([x, y, 0.1], [0.0, 0.0, 1.0], [x, y]));
            vertices.push(BinaryVertex::new([next.0, next.1, 0.1], [0.0, 0.0, 1.0], [next.0, next.1]));
            vertices.push(BinaryVertex::new([0.2, 0.5, 0.1], [0.0, 0.0, 1.0], [0.2, 0.5]));
            
            // Back triangle
            vertices.push(BinaryVertex::new([x, y, -0.1], [0.0, 0.0, -1.0], [x, y]));
            vertices.push(BinaryVertex::new([0.2, 0.5, -0.1], [0.0, 0.0, -1.0], [0.2, 0.5]));
            vertices.push(BinaryVertex::new([next.0, next.1, -0.1], [0.0, 0.0, -1.0], [next.0, next.1]));
        }
        
        // Letter "X" - two crossing bars
        let x_offset = 0.7;
        let bar_width = 0.15;
        
        // Bar 1 (bottom-left to top-right)
        Self::add_bar(&mut vertices, x_offset, 0.0, x_offset + 1.0, 1.0, bar_width);
        
        // Bar 2 (top-left to bottom-right)
        Self::add_bar(&mut vertices, x_offset, 1.0, x_offset + 1.0, 0.0, bar_width);
        
        vertices
    }

    fn add_bar(vertices: &mut Vec<BinaryVertex>, x1: f32, y1: f32, x2: f32, y2: f32, width: f32) {
        // Calculate perpendicular offset
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        let px = -dy / len * width * 0.5;
        let py = dx / len * width * 0.5;
        
        // Front face quad (two triangles)
        let corners = [
            [x1 + px, y1 + py, 0.1],
            [x1 - px, y1 - py, 0.1],
            [x2 + px, y2 + py, 0.1],
            [x2 - px, y2 - py, 0.1],
        ];
        
        vertices.push(BinaryVertex::new(corners[0], [0.0, 0.0, 1.0], [0.0, 0.0]));
        vertices.push(BinaryVertex::new(corners[1], [0.0, 0.0, 1.0], [0.0, 1.0]));
        vertices.push(BinaryVertex::new(corners[2], [0.0, 0.0, 1.0], [1.0, 0.0]));
        
        vertices.push(BinaryVertex::new(corners[1], [0.0, 0.0, 1.0], [0.0, 1.0]));
        vertices.push(BinaryVertex::new(corners[3], [0.0, 0.0, 1.0], [1.0, 1.0]));
        vertices.push(BinaryVertex::new(corners[2], [0.0, 0.0, 1.0], [1.0, 0.0]));
    }

    /// Get binary mesh data (ready for GPU upload)
    pub fn to_binary() -> Vec<u8> {
        let vertices = Self::generate_vertices();
        bytemuck::cast_slice(&vertices).to_vec()
    }
}

// ============================================================================
// 3D Scene Environment
// ============================================================================

/// Environment lighting data
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct EnvironmentLight {
    /// Light direction (normalized)
    pub direction: [f32; 4],
    /// Light color (RGB + intensity)
    pub color: [f32; 4],
    /// Ambient color
    pub ambient: [f32; 4],
}

impl EnvironmentLight {
    /// Default dawn lighting
    pub const fn dawn() -> Self {
        Self {
            direction: [0.5, 0.8, 0.3, 0.0],
            color: [1.0, 0.8, 0.6, 1.0], // Warm sunrise
            ambient: [0.1, 0.15, 0.2, 1.0], // Cool blue ambient
        }
    }

    /// Night mode lighting
    pub const fn night() -> Self {
        Self {
            direction: [0.0, 1.0, 0.0, 0.0],
            color: [0.3, 0.4, 0.8, 0.5], // Cool moonlight
            ambient: [0.05, 0.05, 0.1, 1.0],
        }
    }
}

/// Camera state
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Camera {
    /// View matrix
    pub view: [[f32; 4]; 4],
    /// Projection matrix
    pub projection: [[f32; 4]; 4],
    /// Camera position
    pub position: [f32; 4],
}

impl Camera {
    /// Create perspective camera
    pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32, position: Vec3, target: Vec3) -> Self {
        let proj = Mat4::perspective_rh(fov.to_radians(), aspect, near, far);
        let view = Mat4::look_at_rh(position, target, Vec3::Y);
        
        Self {
            view: view.to_cols_array_2d(),
            projection: proj.to_cols_array_2d(),
            position: [position.x, position.y, position.z, 1.0],
        }
    }

    /// Default camera for hero section
    pub fn hero_default() -> Self {
        Self::perspective(
            45.0,
            16.0 / 9.0,
            0.1,
            100.0,
            Vec3::new(0.0, 0.5, 3.0),
            Vec3::new(0.5, 0.5, 0.0),
        )
    }
}

// ============================================================================
// Scene State (stored in AppState)
// ============================================================================

/// 3D scene state
pub struct Scene3D {
    /// Camera
    pub camera: Camera,
    /// Environment lighting
    pub light: EnvironmentLight,
    /// DX logo transform
    pub logo_transform: BinaryTransform,
    /// Animation time
    pub time: f32,
}

impl Scene3D {
    /// Create new scene
    pub fn new() -> Self {
        Self {
            camera: Camera::hero_default(),
            light: EnvironmentLight::dawn(),
            logo_transform: BinaryTransform::identity(),
            time: 0.0,
        }
    }

    /// Update scene (called every frame)
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        
        // Slowly rotate the logo
        let rotation = Quat::from_rotation_y(self.time * 0.5);
        self.logo_transform = BinaryTransform::from_trs(
            Vec3::new(0.5, 0.5, 0.0),
            rotation,
            Vec3::splat(1.0),
        );
    }

    /// Get GPU uniform data
    pub fn get_uniforms(&self) -> SceneUniforms {
        SceneUniforms {
            view: self.camera.view,
            projection: self.camera.projection,
            model: self.logo_transform.matrix,
            light_dir: self.light.direction,
            light_color: self.light.color,
            ambient: self.light.ambient,
            time: [self.time, 0.0, 0.0, 0.0],
        }
    }
}

impl Default for Scene3D {
    fn default() -> Self {
        Self::new()
    }
}

/// Scene uniforms for GPU upload (256 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SceneUniforms {
    pub view: [[f32; 4]; 4],
    pub projection: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub light_dir: [f32; 4],
    pub light_color: [f32; 4],
    pub ambient: [f32; 4],
    pub time: [f32; 4],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_size() {
        assert_eq!(std::mem::size_of::<BinaryVertex>(), 32);
    }

    #[test]
    fn test_transform_size() {
        assert_eq!(std::mem::size_of::<BinaryTransform>(), 64);
    }

    #[test]
    fn test_uniforms_size() {
        assert_eq!(std::mem::size_of::<SceneUniforms>(), 256);
    }

    #[test]
    fn test_dx_logo_generation() {
        let vertices = DxLogoMesh::generate_vertices();
        assert!(!vertices.is_empty());
        
        let binary = DxLogoMesh::to_binary();
        assert_eq!(binary.len(), vertices.len() * 32);
    }
}
