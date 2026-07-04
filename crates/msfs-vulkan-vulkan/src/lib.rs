//! Read-only Vulkan capability probing.

#![allow(unsafe_code)]

use std::ffi::CStr;

use anyhow::{Context, Result};
use ash::vk;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProbeReport {
    pub loader_api_version: String,
    pub devices: Vec<DeviceReport>,
}

#[derive(Debug, Clone, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct DeviceReport {
    pub name: String,
    pub device_type: String,
    pub api_version: String,
    pub driver_version: u32,
    pub device_local_memory_mib: u64,
    pub graphics_queue: bool,
    pub swapchain: bool,
    pub descriptor_indexing: bool,
    pub runtime_descriptor_array: bool,
    pub descriptor_binding_partially_bound: bool,
    pub descriptor_binding_update_unused_while_pending: bool,
    pub descriptor_binding_variable_descriptor_count: bool,
    pub max_update_after_bind_descriptors: u32,
    pub ray_tracing_pipeline: bool,
    pub acceleration_structure: bool,
    pub basic_vkd3d_candidate: bool,
}

/// Query the Vulkan loader and physical devices without enabling device extensions.
///
/// # Errors
///
/// Returns an error when the loader is absent, instance creation fails, or driver queries fail.
#[allow(clippy::too_many_lines)]
pub fn probe() -> Result<ProbeReport> {
    // SAFETY: ash loads function pointers from the system Vulkan loader and validates required symbols.
    let entry = unsafe { ash::Entry::load() }.context(
        "Vulkan loader not found; install a current GPU driver that provides vulkan-1.dll",
    )?;
    // SAFETY: This only queries the loader and does not retain borrowed pointers.
    let loader_version = unsafe { entry.try_enumerate_instance_version() }
        .context("failed to query Vulkan loader version")?
        .unwrap_or(vk::API_VERSION_1_0);

    let application_name = c"msfs-vulkan-probe";
    let application_info = vk::ApplicationInfo::default()
        .application_name(application_name)
        .application_version(1)
        .engine_name(application_name)
        .engine_version(1)
        .api_version(loader_version.min(vk::API_VERSION_1_3));
    let create_info = vk::InstanceCreateInfo::default().application_info(&application_info);
    // SAFETY: create_info and its pointees live through the call; no custom allocator is used.
    let instance = unsafe { entry.create_instance(&create_info, None) }
        .context("failed to create a Vulkan instance")?;

    let result: Result<Vec<DeviceReport>> = (|| {
        // SAFETY: instance is valid for the duration of every query below.
        let physical_devices = unsafe { instance.enumerate_physical_devices() }
            .context("failed to enumerate Vulkan physical devices")?;
        let mut devices = Vec::with_capacity(physical_devices.len());
        for physical_device in physical_devices {
            // SAFETY: physical_device was returned by this instance.
            let properties = unsafe { instance.get_physical_device_properties(physical_device) };
            // SAFETY: device_name is a fixed NUL-terminated C array supplied by the driver.
            let name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            // SAFETY: physical_device was returned by this instance.
            let memory = unsafe { instance.get_physical_device_memory_properties(physical_device) };
            let device_local_memory = memory.memory_heaps[..memory.memory_heap_count as usize]
                .iter()
                .filter(|heap| heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL))
                .map(|heap| heap.size)
                .sum::<u64>();
            // SAFETY: physical_device was returned by this instance.
            let queues =
                unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
            let graphics_queue = queues
                .iter()
                .any(|queue| queue.queue_flags.contains(vk::QueueFlags::GRAPHICS));
            // SAFETY: physical_device was returned by this instance.
            let extensions =
                unsafe { instance.enumerate_device_extension_properties(physical_device) }
                    .context("failed to enumerate Vulkan device extensions")?;
            let mut swapchain = false;
            let mut ray_tracing_pipeline = false;
            let mut acceleration_structure = false;

            for extension in &extensions {
                // SAFETY: extension_name is a fixed NUL-terminated C array supplied by the driver.
                let ext_name = unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) };
                if ext_name == ash::khr::swapchain::NAME {
                    swapchain = true;
                } else if ext_name == c"VK_KHR_ray_tracing_pipeline" {
                    ray_tracing_pipeline = true;
                } else if ext_name == c"VK_KHR_acceleration_structure" {
                    acceleration_structure = true;
                }
            }

            let mut vulkan12_features = vk::PhysicalDeviceVulkan12Features::default();
            let mut features2 =
                vk::PhysicalDeviceFeatures2::default().push_next(&mut vulkan12_features);
            // SAFETY: output structures are valid and physical_device belongs to the instance.
            unsafe { instance.get_physical_device_features2(physical_device, &mut features2) };

            let mut vulkan12_properties = vk::PhysicalDeviceVulkan12Properties::default();
            let mut properties2 =
                vk::PhysicalDeviceProperties2::default().push_next(&mut vulkan12_properties);
            // SAFETY: output structures are valid and physical_device belongs to the instance.
            unsafe { instance.get_physical_device_properties2(physical_device, &mut properties2) };

            let descriptor_indexing = vulkan12_features.descriptor_indexing == vk::TRUE;
            let runtime_descriptor_array = vulkan12_features.runtime_descriptor_array == vk::TRUE;
            let partially_bound = vulkan12_features.descriptor_binding_partially_bound == vk::TRUE;
            let update_unused =
                vulkan12_features.descriptor_binding_update_unused_while_pending == vk::TRUE;
            let variable_count =
                vulkan12_features.descriptor_binding_variable_descriptor_count == vk::TRUE;
            let api_13 = properties.api_version >= vk::API_VERSION_1_3;
            let max_descriptors =
                vulkan12_properties.max_update_after_bind_descriptors_in_all_pools;

            devices.push(DeviceReport {
                name,
                device_type: format!("{:?}", properties.device_type),
                api_version: version_string(properties.api_version),
                driver_version: properties.driver_version,
                device_local_memory_mib: device_local_memory / (1024 * 1024),
                graphics_queue,
                swapchain,
                descriptor_indexing,
                runtime_descriptor_array,
                descriptor_binding_partially_bound: partially_bound,
                descriptor_binding_update_unused_while_pending: update_unused,
                descriptor_binding_variable_descriptor_count: variable_count,
                max_update_after_bind_descriptors: max_descriptors,
                ray_tracing_pipeline,
                acceleration_structure,
                basic_vkd3d_candidate: api_13
                    && graphics_queue
                    && swapchain
                    && descriptor_indexing
                    && runtime_descriptor_array
                    && partially_bound
                    && update_unused
                    && variable_count
                    && max_descriptors >= 1_000_000,
            });
        }
        Ok(devices)
    })();

    // SAFETY: all child queries have completed and no handles derived from the instance remain.
    unsafe { instance.destroy_instance(None) };
    Ok(ProbeReport {
        loader_api_version: version_string(loader_version),
        devices: result?,
    })
}

fn version_string(version: u32) -> String {
    format!(
        "{}.{}.{}",
        vk::api_version_major(version),
        vk::api_version_minor(version),
        vk::api_version_patch(version)
    )
}
