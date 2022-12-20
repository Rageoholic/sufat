use std::ffi::CStr;

use ash::{
    extensions::ext::DebugUtils,
    vk::{self, ApplicationInfo},
    Entry, Instance,
};
use cstr::cstr;
use raw_window_handle::HasRawDisplayHandle;
use winit::window::Window;

#[allow(dead_code)]
pub struct RenderContext {
    entry: Entry,
    instance: Instance,
}

#[derive(Debug)]
pub enum RenderContextError {
    MissingExtension,
    MissingLayer,
    MissingExtensionAndLayer,
    UnableToLoadLib,
    InstanceCreationFailed,
}

impl RenderContext {
    pub fn new(window: &Window) -> Result<RenderContext, RenderContextError> {
        //SAFETY: Admittedly not actually safe since someone can make a vulkan
        //lib that on startup scribbles all over our memory or some nonsense but
        //this is a game so. Whatever. Risk I can take.
        match unsafe { Entry::load() } {
            Ok(entry) => {
                let vk_version = entry
                    .try_enumerate_instance_version()
                    .unwrap()
                    .unwrap_or(vk::make_api_version(0, 1, 0, 0));
                let app_info = ApplicationInfo::builder()
                    .api_version(vk_version)
                    .engine_name(cstr!("Rageware"))
                    .engine_version(0)
                    .application_name(cstr!("sufat"))
                    .application_version(0)
                    .build();
                let mut required_extensions =
                    ash_window::enumerate_required_extensions(
                        window.raw_display_handle(),
                    )
                    .unwrap()
                    .to_vec();
                required_extensions.push(DebugUtils::name().as_ptr());

                let ext_props = entry
                    .enumerate_instance_extension_properties(None)
                    .unwrap();
                let exts_missing: Vec<*const i8> = required_extensions
                    .iter()
                    .map(|needle_extension_name| {
                        ext_props
                            .iter()
                            //SAFETY: Fine because all our strings are null
                            //terminated
                            .find_map(|ext_prop| unsafe {
                                CStr::from_ptr(*needle_extension_name)
                                    .cmp(CStr::from_ptr(
                                        ext_prop.extension_name.as_ptr(),
                                    ))
                                    .is_eq()
                                    .then_some(())
                            })
                            .ok_or(needle_extension_name)
                    })
                    .filter_map(|i| i.err().copied())
                    .collect();

                let exts_missing = if exts_missing.len() > 0 {
                    for ext in exts_missing {
                        //SAFETY: Fine because all these strings are null
                        //terminated
                        log::error!("Missing extension {:?}", unsafe {
                            CStr::from_ptr(ext)
                        });
                    }
                    true
                } else {
                    false
                };

                let debug_layer_names =
                    [cstr!("VK_LAYER_KHRONOS_validation").as_ptr()];

                let layer_props =
                    entry.enumerate_instance_layer_properties().unwrap();
                let layers_missing: Vec<*const i8> = debug_layer_names
                    .iter()
                    .map(|needle_layer_name| {
                        layer_props
                            .iter()
                            .find_map(|m| -> Option<()> {
                                //SAFETY: Legal because m.layer_name is always
                                //null terminated and we make our needle name
                                //from cstr!
                                unsafe {
                                    CStr::from_ptr(*needle_layer_name)
                                        .cmp(CStr::from_ptr(
                                            m.layer_name.as_ptr(),
                                        ))
                                        .is_eq()
                                        .then_some(())
                                }
                            })
                            .ok_or(needle_layer_name)
                    })
                    .filter_map(|i| i.err().copied())
                    .collect();

                let layers_missing = if layers_missing.len() != 0 {
                    for missing_layer_name in layers_missing {
                        //SAFETY: We build these off of cstr! so we're fine
                        log::error!("Missing layer {:?}", unsafe {
                            CStr::from_ptr(missing_layer_name)
                        })
                    }
                    true
                } else {
                    false
                };
                if layers_missing && exts_missing {
                    Err(RenderContextError::MissingExtensionAndLayer)
                } else if layers_missing {
                    Err(RenderContextError::MissingLayer)
                } else if exts_missing {
                    Err(RenderContextError::MissingExtension)
                } else {
                    log::debug!("Successfully found all layers");

                    let create_info = vk::InstanceCreateInfo::builder()
                        .application_info(&app_info)
                        .enabled_extension_names(&required_extensions)
                        .enabled_layer_names(&debug_layer_names)
                        .build();

                    //SAFETY: we constructed create_instance from a builder
                    //using correct parameters so it should be correct too
                    match unsafe { entry.create_instance(&create_info, None) } {
                        Ok(instance) => Ok(RenderContext { entry, instance }),
                        Err(_) => {
                            Err(RenderContextError::InstanceCreationFailed)
                        }
                    }
                }
            }
            Err(_) => Err(RenderContextError::UnableToLoadLib),
        }
    }
}
