/*
This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use std::{borrow::Cow, ffi::CStr, sync::Arc};

use ash::{
    extensions::{ext::DebugUtils, khr::Surface},
    vk::{
        self, ApplicationInfo, DebugUtilsMessageSeverityFlagsEXT,
        DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCallbackDataEXT,
        DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerEXT, SurfaceKHR,
    },
    Entry, Instance,
};
use cstr::cstr;
use log::Level;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;

#[allow(dead_code)]
pub struct RenderContext {
    entry: Entry,
    instance: Instance,
    debug_callback: Option<DebugUtilsMessengerEXT>,
    debug_utils_loader: DebugUtils,
    surface: SurfaceKHR,
    surface_callbacks: Surface,
    //hold on to the window as we need to make sure it is not dropped under any
    //circumstances until we drop this Arc
    _window: Arc<Window>,
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
    pub fn new(
        window: Arc<Window>,
    ) -> Result<RenderContext, RenderContextError> {
        //SAFETY: Admittedly not actually safe since someone can make a vulkan
        //lib that on startup scribbles all over our memory or some nonsense but
        //this is a game so. Whatever. Risk I can take.
        match unsafe { Entry::load() } {
            Err(_) => Err(RenderContextError::UnableToLoadLib),

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
                        Err(_) => {
                            log::error!(
                                "We got to creating an instance but it failed\
                                for some reason"
                            );
                            Err(RenderContextError::InstanceCreationFailed)
                        }

                        Ok(instance) => {
                            log::info!("Successfully created instance");

                            let mut debug_messenger_log_level =
                                DebugUtilsMessageSeverityFlagsEXT::empty();
                            let log_level = log::max_level();
                            if log_level >= Level::Error {
                                debug_messenger_log_level |=
                                    DebugUtilsMessageSeverityFlagsEXT::ERROR
                            }
                            if log_level >= Level::Warn {
                                debug_messenger_log_level |=
                                    DebugUtilsMessageSeverityFlagsEXT::WARNING
                            }
                            if log_level >= Level::Info {
                                debug_messenger_log_level |=
                                    DebugUtilsMessageSeverityFlagsEXT::INFO
                            }
                            if log_level >= Level::Trace {
                                debug_messenger_log_level |=
                                    DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                            }

                            let debug_utils_loader =
                                DebugUtils::new(&entry, &instance);

                            let debug_info =
                                DebugUtilsMessengerCreateInfoEXT::builder()
                                    .message_severity(
                                        debug_messenger_log_level,
                                    ).message_type(DebugUtilsMessageTypeFlagsEXT::GENERAL
                                         | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE |
                                             DebugUtilsMessageTypeFlagsEXT::VALIDATION)
                                             .pfn_user_callback(Some(vulkan_debug_callback))
                                             .build();

                            let debug_callback = unsafe {
                                debug_utils_loader
                                    .create_debug_utils_messenger(
                                        &debug_info,
                                        None,
                                    )
                                    .ok()
                            };

                            test_debug_callback(&debug_utils_loader);
                            let surface_callbacks =
                                Surface::new(&entry, &instance);
                            let surface = unsafe {
                                ash_window::create_surface(
                                    &entry,
                                    &instance,
                                    window.raw_display_handle(),
                                    window.raw_window_handle(),
                                    None,
                                )
                            }
                            .unwrap();
                            Ok(RenderContext {
                                entry,
                                instance,
                                debug_callback,
                                surface,
                                debug_utils_loader,
                                _window: window,
                                surface_callbacks,
                            })
                        }
                    }
                }
            }
        }
    }
}
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    //pointer guaranteed to be valid by API rules since this isn't exported
    let callback_data = unsafe { *p_callback_data };
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        //SAFETY: strings from vk are null terminated
        unsafe {
            CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
        }
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        //SAFETY: strings from vk are null terminated
        unsafe { CStr::from_ptr(callback_data.p_message).to_string_lossy() }
    };

    match message_severity {
        DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            log::error!(
                "vulkan debug utils.\n\
            \ttype: {:?}\n\
            \tid_name: {:?}\n\
            \tid_num: {:?}\n\
            \tmessage: {:?}",
                message_type,
                message_id_name,
                message_id_number,
                message
            )
        }
        DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log::warn!(
                "vulkan debug utils.\n\
                \ttype: {:?}\n\
                \tid_name: {:?}\n\
                \tid_num: {:?}\n\
                \tmessage: {:?}",
                message_type,
                message_id_name,
                message_id_number,
                message
            )
        }
        DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log::info!(
                "vulkan debug utils.\n\
                \ttype: {:?}\n\
                \tid_name: {:?}\n\
                \tid_num: {:?}\n\
                \tmessage: {:?}",
                message_type,
                message_id_name,
                message_id_number,
                message
            )
        }
        DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            log::trace!(
                "vulkan debug utils.\n\
                \ttype: {:?}\n\
                \tid_name: {:?}\n\
                \tid_num: {:?}\n\
                \tmessage: {:?}",
                message_type,
                message_id_name,
                message_id_number,
                message
            )
        }
        _ => {
            unreachable!()
        }
    }

    vk::FALSE
}

fn test_debug_callback(debug_utils_loader: &DebugUtils) {
    let callback_data = DebugUtilsMessengerCallbackDataEXT::builder()
        .message(cstr!("test"))
        .message_id_name(cstr!("testing"))
        .message_id_number(1)
        .build();

    //SAFETY: We built callback_data with a builder and valid cstrs
    unsafe {
        debug_utils_loader.submit_debug_utils_message(
            DebugUtilsMessageSeverityFlagsEXT::ERROR,
            DebugUtilsMessageTypeFlagsEXT::GENERAL,
            &callback_data,
        );
        debug_utils_loader.submit_debug_utils_message(
            DebugUtilsMessageSeverityFlagsEXT::WARNING,
            DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            &callback_data,
        );
        debug_utils_loader.submit_debug_utils_message(
            DebugUtilsMessageSeverityFlagsEXT::INFO,
            DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            &callback_data,
        );
        debug_utils_loader.submit_debug_utils_message(
            DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
            DebugUtilsMessageTypeFlagsEXT::GENERAL,
            &callback_data,
        );
    }
}

impl Drop for RenderContext {
    fn drop(&mut self) {
        log::info!("Destroying render context");
        //SAFETY: We correctly construct this in new
        self.debug_callback.map(|debug_callback| unsafe {
            self.debug_utils_loader
                .destroy_debug_utils_messenger(debug_callback, None)
        });

        unsafe {
            self.surface_callbacks.destroy_surface(self.surface, None);
        }

        //SAFETY: We correctly construct this in new
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}
