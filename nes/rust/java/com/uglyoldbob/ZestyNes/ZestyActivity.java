package com.uglyoldbob.ZestyNes;

import android.app.NativeActivity;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.util.Log;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;

public class ZestyActivity extends NativeActivity {
    private static final int PICK_ROM = 1001;

    static {
        System.loadLibrary("zesty_nes");
    }

    public void openRomPicker() {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.setType("*/*");

        startActivityForResult(intent, PICK_ROM);
    }

    private byte[] readRom(Uri uri) throws IOException {
        try (
            InputStream input =
                getContentResolver().openInputStream(uri);
            ByteArrayOutputStream output =
                new ByteArrayOutputStream()
        ) {
            byte[] buffer = new byte[64 * 1024];
    
            int n;
            while ((n = input.read(buffer)) != -1) {
                output.write(buffer, 0, n);
            }
    
            return output.toByteArray();
        }
    }

    private String getRomName(Uri uri) {
        String name = null;

        try (Cursor cursor = getContentResolver().query(
                uri,
                new String[] { OpenableColumns.DISPLAY_NAME },
                null,
                null,
                null)) {

            if (cursor != null && cursor.moveToFirst()) {
                int index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);

                if (index >= 0) {
                    name = cursor.getString(index);
                }
            }
        }

        if (name == null) {
            name = "Unknown ROM";
        }

        // Remove the extension.
        int dot = name.lastIndexOf('.');
        if (dot > 0) {
            name = name.substring(0, dot);
        }

        return name;
    }

    @Override
    protected void onActivityResult(
            int requestCode,
            int resultCode,
            Intent data) {

        super.onActivityResult(requestCode, resultCode, data);

        if (requestCode != PICK_ROM ||
            resultCode != RESULT_OK ||
            data == null) {
            return;
        }

        Uri uri = data.getData();

        if (uri == null) {
            return;
        }

        int flags = data.getFlags() & Intent.FLAG_GRANT_READ_URI_PERMISSION;

        getContentResolver().takePersistableUriPermission(uri, flags);

        try {
            byte[] rom = readRom(uri);
            String name = getRomName(uri);
            send_user_selected_rom(rom, name);
        } catch (IOException e) {
            Log.e("ZestyNes", "Failed to read ROM", e);
        }
    }

    public static native void send_user_selected_rom(byte[] rom, String name);
}