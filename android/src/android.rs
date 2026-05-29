use egui_winit::winit;
use jni::objects::{JObject, JValue};

pub fn request_bluetooth_connect(
    app: &winit::platform::android::activity::AndroidApp,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get JVM + attach thread
    let vm = app.vm_as_ptr();

    let vm = unsafe { jni::JavaVM::from_raw(vm.cast())? };

    let mut env = vm.attach_current_thread()?;

    // Get Activity
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };

    // Check SDK version
    let version_class = env.find_class("android/os/Build$VERSION")?;

    let sdk_int = env.get_static_field(version_class, "SDK_INT", "I")?.i()?;

    // Android 12+
    if sdk_int < 31 {
        return Ok(());
    }

    // android.Manifest.permission.BLUETOOTH_CONNECT
    let manifest_permission = env.find_class("android/Manifest$permission")?;

    let bluetooth_connect = env
        .get_static_field(
            manifest_permission,
            "BLUETOOTH_CONNECT",
            "Ljava/lang/String;",
        )?
        .l()?;

    // activity.checkSelfPermission(...)
    let result = env
        .call_method(
            &activity,
            "checkSelfPermission",
            "(Ljava/lang/String;)I",
            &[JValue::Object(&bluetooth_connect)],
        )?
        .i()?;

    // PackageManager.PERMISSION_GRANTED == 0
    if result == 0 {
        log::error!("Bluetooth permission already granted");
        return Ok(());
    }

    // Create String[]
    let string_class = env.find_class("java/lang/String")?;

    let permissions = env.new_object_array(1, string_class, JObject::null())?;

    env.set_object_array_element(&permissions, 0, &bluetooth_connect)?;

    // requestPermissions(String[], int)
    env.call_method(
        &activity,
        "requestPermissions",
        "([Ljava/lang/String;I)V",
        &[JValue::Object(&permissions), JValue::Int(1001)],
    )?;

    log::error!("Requested BLUETOOTH_CONNECT permission");

    Ok(())
}
