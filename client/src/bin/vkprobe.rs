//! vkprobe: lists the Vulkan devices this process can see, their queue families and the
//! external-memory extensions yserver cares about.

use ash::vk;

const EXTS: &[&str] = &[
    "VK_ANDROID_external_memory_android_hardware_buffer",
    "VK_EXT_external_memory_dma_buf",
    "VK_KHR_external_memory_fd",
    "VK_EXT_image_drm_format_modifier",
    "VK_EXT_physical_device_drm",
    "VK_KHR_external_semaphore_fd",
    "VK_KHR_timeline_semaphore",
];

fn main() {
    let entry = match unsafe { ash::Entry::load() } {
        Ok(e) => e,
        Err(e) => {
            println!("vkprobe: cannot load libvulkan: {e}");
            std::process::exit(1);
        }
    };
    let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_1);
    let ci = vk::InstanceCreateInfo::default().application_info(&app);
    let instance = match unsafe { entry.create_instance(&ci, None) } {
        Ok(i) => i,
        Err(e) => {
            println!("vkprobe: vkCreateInstance failed: {e}");
            std::process::exit(1);
        }
    };
    let devices = unsafe { instance.enumerate_physical_devices() }.unwrap_or_default();
    println!("vkprobe: {} physical device(s)", devices.len());
    for pd in devices {
        let p = unsafe { instance.get_physical_device_properties(pd) };
        let name = p.device_name_as_c_str().map(|c| c.to_string_lossy().into_owned()).unwrap_or_default();
        println!(
            "device {name:?} type {:?} api {}.{}.{} driver 0x{:x} vendor 0x{:x}",
            p.device_type,
            vk::api_version_major(p.api_version),
            vk::api_version_minor(p.api_version),
            vk::api_version_patch(p.api_version),
            p.driver_version,
            p.vendor_id
        );
        let qf = unsafe { instance.get_physical_device_queue_family_properties(pd) };
        for (i, q) in qf.iter().enumerate() {
            println!("  queue family {i}: count {} flags {:?}", q.queue_count, q.queue_flags);
        }
        let have: Vec<String> = unsafe { instance.enumerate_device_extension_properties(pd) }
            .unwrap_or_default()
            .iter()
            .filter_map(|e| e.extension_name_as_c_str().ok().map(|c| c.to_string_lossy().into_owned()))
            .collect();
        println!("  {} device extensions", have.len());
        for want in EXTS {
            println!("  {want}: {}", if have.iter().any(|h| h == want) { "yes" } else { "no" });
        }
    }
    unsafe { instance.destroy_instance(None) };
}
