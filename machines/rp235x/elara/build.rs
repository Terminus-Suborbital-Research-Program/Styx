use aether_core::{matrix, math::{Matrix, Vector}};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

const ELARA_GEOMETRY_CONFIG_PATH: &str = "../../../../CrabPilot/inputs/elara/geometry.toml";

#[derive(Deserialize)]
struct SpacecraftConfig {
    vehicle: VehicleConfig,
    control: ControlConfig,
}

#[derive(Deserialize)]
struct VehicleConfig {
    inertia: InertiaConfig,
}

#[derive(Deserialize)]
struct InertiaConfig {
    value: [[f64; 3]; 3],
}

#[derive(Deserialize)]
struct ControlConfig {
    constraints: ControlConstraints,
    #[serde(default)]
    maneuver: ManeuverConfig,
}

#[derive(Deserialize)]
struct ControlConstraints {
    max_desired_body_rate_dps: [f64; 3],
}

#[derive(Deserialize)]
struct ManeuverConfig {
    segment_duration_s: f64,
    zero_rate_attitude_deg: [f64; 3],
    roll_attitude_deg: [f64; 3],
    pitch_attitude_deg: [f64; 3],
    yaw_attitude_deg: [f64; 3],
    final_roll_rate_dps: f64,
}

impl Default for ManeuverConfig {
    fn default() -> Self {
        Self {
            segment_duration_s: 20.0,
            zero_rate_attitude_deg: [0.0, 0.0, 0.0],
            roll_attitude_deg: [90.0, 0.0, 0.0],
            pitch_attitude_deg: [90.0, 90.0, 0.0],
            yaw_attitude_deg: [90.0, 90.0, 90.0],
            final_roll_rate_dps: 2.5,
        }
    }
}

#[derive(Deserialize)]
struct GeometryConfig {
    #[serde(default)]
    body_frame_rotation_deg: [f64; 3],
    #[serde(default)]
    body_frame_forward_axis: Option<String>,
    #[serde(default)]
    body_frame_up_axis: Option<String>,
    #[serde(default)]
    parts: Vec<GeometryPartConfig>,
}

#[derive(Deserialize)]
struct GeometryPartConfig {
    name: String,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    rotation_assembly_deg: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SignedAxis {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl SignedAxis {
    fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase().replace(' ', "");
        match normalized.as_str() {
            "+x" | "x" | "posx" | "+1x" => Some(Self::PosX),
            "-x" | "negx" | "-1x" => Some(Self::NegX),
            "+y" | "y" | "posy" | "+1y" => Some(Self::PosY),
            "-y" | "negy" | "-1y" => Some(Self::NegY),
            "+z" | "z" | "posz" | "+1z" => Some(Self::PosZ),
            "-z" | "negz" | "-1z" => Some(Self::NegZ),
            _ => None,
        }
    }

    fn vec(self) -> [f64; 3] {
        match self {
            Self::PosX => [1.0, 0.0, 0.0],
            Self::NegX => [-1.0, 0.0, 0.0],
            Self::PosY => [0.0, 1.0, 0.0],
            Self::NegY => [0.0, -1.0, 0.0],
            Self::PosZ => [0.0, 0.0, 1.0],
            Self::NegZ => [0.0, 0.0, -1.0],
        }
    }

    fn base_axis(self) -> usize {
        match self {
            Self::PosX | Self::NegX => 0,
            Self::PosY | Self::NegY => 1,
            Self::PosZ | Self::NegZ => 2,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ResolvedGeometryPart {
    rotation_matrix: Matrix<f64, 3, 3>,
}

fn rotation_matrix_xyz_deg(rotation_deg: [f64; 3]) -> Matrix<f64, 3, 3> {
    let [roll_deg, pitch_deg, yaw_deg] = rotation_deg;
    let (roll, pitch, yaw) = (
        roll_deg.to_radians(),
        pitch_deg.to_radians(),
        yaw_deg.to_radians(),
    );
    let (sr, cr) = roll.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let (sy, cy) = yaw.sin_cos();

    Matrix::new([
        [cy * cp, cy * sp * sr - sy * cr, cy * sp * cr + sy * sr],
        [sy * cp, sy * sp * sr + cy * cr, sy * sp * cr - cy * sr],
        [-sp, cp * sr, cp * cr],
    ])
}

fn normalize_vector(vector: Vector<f64, 3>) -> Vector<f64, 3> {
    let (normalized, norm) = vector.try_normalize();
    if norm > 1.0e-12 {
        normalized
    } else {
        Vector::new([0.0, 0.0, 1.0])
    }
}

fn body_axis_mapping_rotation_matrix(
    forward_axis: SignedAxis,
    up_axis: SignedAxis,
) -> Matrix<f64, 3, 3> {
    assert!(
        forward_axis.base_axis() != up_axis.base_axis(),
        "body axis declaration is invalid: forward and up axes are collinear"
    );

    let ex_body_in_mesh = Vector::new(forward_axis.vec());
    let ez_body_in_mesh = Vector::new(up_axis.vec());
    let ey_body_in_mesh = ez_body_in_mesh.cross(&ex_body_in_mesh);

    matrix![
        [ex_body_in_mesh[0], ex_body_in_mesh[1], ex_body_in_mesh[2]],
        [ey_body_in_mesh[0], ey_body_in_mesh[1], ey_body_in_mesh[2]],
        [ez_body_in_mesh[0], ez_body_in_mesh[1], ez_body_in_mesh[2]],
    ]
}

fn resolve_geometry_part_rotations(config: &GeometryConfig) -> Vec<ResolvedGeometryPart> {
    let mut name_to_index = HashMap::with_capacity(config.parts.len());
    for (index, part) in config.parts.iter().enumerate() {
        let previous = name_to_index.insert(part.name.clone(), index);
        assert!(
            previous.is_none(),
            "geometry part name '{}' is duplicated",
            part.name
        );
    }

    let mut resolved = vec![None; config.parts.len()];
    let mut visiting = vec![false; config.parts.len()];
    for index in 0..config.parts.len() {
        resolve_geometry_part_rotation(index, config, &name_to_index, &mut resolved, &mut visiting);
    }

    resolved
        .into_iter()
        .map(|entry| entry.expect("internal error: unresolved geometry part rotation"))
        .collect()
}

fn resolve_geometry_part_rotation(
    index: usize,
    config: &GeometryConfig,
    name_to_index: &HashMap<String, usize>,
    resolved: &mut [Option<ResolvedGeometryPart>],
    visiting: &mut [bool],
) -> ResolvedGeometryPart {
    if let Some(part) = resolved[index] {
        return part;
    }
    assert!(
        !visiting[index],
        "geometry parent cycle detected at part '{}'",
        config.parts[index].name
    );

    visiting[index] = true;
    let part = &config.parts[index];
    let local_rotation = rotation_matrix_xyz_deg(part.rotation_assembly_deg);

    let resolved_rotation = if let Some(parent_name) = part.parent.as_deref() {
        let parent_index = *name_to_index.get(parent_name).unwrap_or_else(|| {
            panic!(
                "geometry part '{}' references missing parent '{}'",
                part.name, parent_name
            )
        });
        assert!(
            parent_index != index,
            "geometry part '{}' cannot be its own parent",
            part.name
        );

        let parent = resolve_geometry_part_rotation(
            parent_index,
            config,
            name_to_index,
            resolved,
            visiting,
        );
        ResolvedGeometryPart {
            rotation_matrix: parent.rotation_matrix * local_rotation,
        }
    } else {
        ResolvedGeometryPart {
            rotation_matrix: local_rotation,
        }
    };

    visiting[index] = false;
    resolved[index] = Some(resolved_rotation);
    resolved_rotation
}

fn load_momentum_wheel_axes_body_xyz(geometry_path: &Path) -> [[f64; 3]; 3] {
    println!("cargo:rerun-if-changed={}", geometry_path.display());

    let contents = fs::read_to_string(geometry_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {}", geometry_path.display(), err));
    let config: GeometryConfig = toml::from_str(&contents)
        .unwrap_or_else(|err| panic!("failed to parse {}: {}", geometry_path.display(), err));
    let resolved_parts = resolve_geometry_part_rotations(&config);

    let axis_rotation = match (
        config.body_frame_forward_axis.as_deref(),
        config.body_frame_up_axis.as_deref(),
    ) {
        (Some(forward), Some(up)) => Some(body_axis_mapping_rotation_matrix(
            SignedAxis::parse(forward).unwrap_or_else(|| {
                panic!("invalid body_frame_forward_axis '{}'", forward)
            }),
            SignedAxis::parse(up)
                .unwrap_or_else(|| panic!("invalid body_frame_up_axis '{}'", up)),
        )),
        (None, None) => None,
        _ => panic!(
            "geometry must define both body_frame_forward_axis and body_frame_up_axis or neither"
        ),
    };
    let body_rotation = if config.body_frame_rotation_deg != [0.0, 0.0, 0.0] {
        Some(rotation_matrix_xyz_deg(config.body_frame_rotation_deg))
    } else {
        None
    };

    let mut ordered_axes = [None; 3];
    for (index, part) in config.parts.iter().enumerate() {
        if part.role.as_deref() != Some("reaction_wheel") {
            continue;
        }

        let mut spin_axis_body = resolved_parts[index].rotation_matrix * Vector::new([0.0, 0.0, 1.0]);
        if let Some(rotation) = axis_rotation {
            spin_axis_body = rotation * spin_axis_body;
        }
        if let Some(rotation) = body_rotation {
            spin_axis_body = rotation * spin_axis_body;
        }
        spin_axis_body = normalize_vector(spin_axis_body);

        let mut dominant_axis = 0usize;
        let mut dominant_magnitude = spin_axis_body[0].abs();
        for axis in 1..3 {
            let magnitude = spin_axis_body[axis].abs();
            if magnitude > dominant_magnitude {
                dominant_axis = axis;
                dominant_magnitude = magnitude;
            }
        }

        assert!(
            dominant_magnitude >= 0.9,
            "reaction wheel '{}' does not align with a spacecraft body axis: {:?}",
            part.name,
            spin_axis_body
        );
        assert!(
            ordered_axes[dominant_axis].is_none(),
            "multiple reaction wheels align with spacecraft body axis {}",
            dominant_axis
        );
        ordered_axes[dominant_axis] = Some(spin_axis_body.data);
    }

    ordered_axes.map(|axis| axis.expect("geometry must define exactly one reaction wheel for each body axis"))
}

fn cargo_rustflags() -> Vec<String> {
    std::env::var_os("CARGO_ENCODED_RUSTFLAGS")
        .map(|flags| {
            flags
                .to_string_lossy()
                .split('\u{1f}')
                .filter(|flag| !flag.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn has_rustflag(flags: &[String], expected: &str) -> bool {
    flags.iter().any(|flag| {
        flag == expected
            || flag == &format!("link-arg={expected}")
            || flag == &format!("-Clink-arg={expected}")
    })
}

fn generate_spacecraft_config(out: &PathBuf) {
    let config_path = PathBuf::from("spacecraft.toml");
    println!("cargo:rerun-if-changed={}", config_path.display());

    let contents = fs::read_to_string(&config_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {}", config_path.display(), err));
    let config: SpacecraftConfig = toml::from_str(&contents)
        .unwrap_or_else(|err| panic!("failed to parse {}: {}", config_path.display(), err));

    let inertia = config.vehicle.inertia.value;
    let inertia_diagonal = [inertia[0][0], inertia[1][1], inertia[2][2]];
    for (index, value) in inertia_diagonal.iter().enumerate() {
        assert!(*value > 0.0, "spacecraft inertia diagonal {index} must be > 0");
    }
    for row in 0..3 {
        for col in 0..3 {
            if row != col {
                assert!(
                    inertia[row][col].abs() <= 1.0e-9,
                    "spacecraft inertia must be diagonal for the current satellite MPC model"
                );
            }
        }
    }

    let max_rate_dps = config.control.constraints.max_desired_body_rate_dps;
    for (index, value) in max_rate_dps.iter().enumerate() {
        assert!(*value > 0.0, "max desired body rate {index} must be > 0 dps");
    }
    let max_rate_rad_s = [
        max_rate_dps[0].to_radians(),
        max_rate_dps[1].to_radians(),
        max_rate_dps[2].to_radians(),
    ];
    let maneuver = config.control.maneuver;
    assert!(
        maneuver.segment_duration_s > 0.0,
        "maneuver segment duration must be > 0 s"
    );
    assert!(
        maneuver.final_roll_rate_dps >= 0.0,
        "maneuver final roll rate must be >= 0 dps"
    );
    assert!(
        maneuver.final_roll_rate_dps <= max_rate_dps[0],
        "maneuver final roll rate must be <= the configured roll-axis max desired body rate"
    );
    let maneuver_attitudes_deg = [
        maneuver.zero_rate_attitude_deg,
        maneuver.roll_attitude_deg,
        maneuver.pitch_attitude_deg,
        maneuver.yaw_attitude_deg,
    ];
    let maneuver_attitudes_rad = maneuver_attitudes_deg.map(|attitude_deg| {
        attitude_deg.map(f64::to_radians)
    });
    let maneuver_final_roll_rate_rad_s = maneuver.final_roll_rate_dps.to_radians();
    let geometry_path = PathBuf::from(ELARA_GEOMETRY_CONFIG_PATH);
    let wheel_axes_body_xyz = load_momentum_wheel_axes_body_xyz(&geometry_path);

    let generated = format!(
        "pub const SPACECRAFT_INERTIA_DIAGONAL_KG_M2: [f32; 3] = [{:.16}, {:.16}, {:.16}];\n\
pub const SPACECRAFT_MOMENTUM_WHEEL_AXES_BODY_XYZ: [[f32; 3]; 3] = [[{:.16}, {:.16}, {:.16}], [{:.16}, {:.16}, {:.16}], [{:.16}, {:.16}, {:.16}]];\n\
pub const MAX_DESIRED_BODY_RATE_DPS: [f32; 3] = [{:.16}, {:.16}, {:.16}];\n\
pub const MAX_DESIRED_BODY_RATE_RAD_S: [f32; 3] = [{:.16}, {:.16}, {:.16}];\n",
        inertia_diagonal[0],
        inertia_diagonal[1],
        inertia_diagonal[2],
        wheel_axes_body_xyz[0][0],
        wheel_axes_body_xyz[0][1],
        wheel_axes_body_xyz[0][2],
        wheel_axes_body_xyz[1][0],
        wheel_axes_body_xyz[1][1],
        wheel_axes_body_xyz[1][2],
        wheel_axes_body_xyz[2][0],
        wheel_axes_body_xyz[2][1],
        wheel_axes_body_xyz[2][2],
        max_rate_dps[0],
        max_rate_dps[1],
        max_rate_dps[2],
        max_rate_rad_s[0],
        max_rate_rad_s[1],
        max_rate_rad_s[2],
    );

    let generated = format!(
        "{}pub const SPACECRAFT_MANEUVER_SEGMENT_DURATION_S: f32 = {:.16};\n\
pub const SPACECRAFT_MANEUVER_ATTITUDE_TARGETS_RAD: [[f32; 3]; 4] = [[{:.16}, {:.16}, {:.16}], [{:.16}, {:.16}, {:.16}], [{:.16}, {:.16}, {:.16}], [{:.16}, {:.16}, {:.16}]];\n\
pub const SPACECRAFT_MANEUVER_FINAL_ROLL_RATE_RAD_S: f32 = {:.16};\n",
        generated,
        maneuver.segment_duration_s,
        maneuver_attitudes_rad[0][0],
        maneuver_attitudes_rad[0][1],
        maneuver_attitudes_rad[0][2],
        maneuver_attitudes_rad[1][0],
        maneuver_attitudes_rad[1][1],
        maneuver_attitudes_rad[1][2],
        maneuver_attitudes_rad[2][0],
        maneuver_attitudes_rad[2][1],
        maneuver_attitudes_rad[2][2],
        maneuver_attitudes_rad[3][0],
        maneuver_attitudes_rad[3][1],
        maneuver_attitudes_rad[3][2],
        maneuver_final_roll_rate_rad_s,
    );

    fs::write(out.join("spacecraft_config.rs"), generated)
        .expect("failed to write generated spacecraft_config.rs");
}

fn main() {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    generate_spacecraft_config(&out);

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_os != "none" || target_arch != "arm" {
        println!("cargo:rerun-if-changed=build.rs");
        return;
    }

    let rustflags = cargo_rustflags();

    let mut memory = File::create(out.join("memory.x")).unwrap();
    memory.write_all(include_bytes!("memory.x")).unwrap();

    println!("cargo:rustc-link-search={}", out.display());
    if !has_rustflag(&rustflags, "--nmagic") {
        println!("cargo:rustc-link-arg=--nmagic");
    }
    if !has_rustflag(&rustflags, "-Tlink.x") {
        println!("cargo:rustc-link-arg=-Tlink.x");
    }
    if !has_rustflag(&rustflags, "-Tdefmt.x") {
        println!("cargo:rustc-link-arg=-Tdefmt.x");
    }

    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}