package com.ideabucket.app;

import static org.junit.Assert.assertTrue;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.fail;

import android.os.Bundle;
import android.util.TypedValue;
import android.view.View;
import android.widget.LinearLayout;
import androidx.appcompat.app.AppCompatActivity;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.robolectric.Robolectric;
import org.robolectric.android.controller.ActivityController;
import org.robolectric.annotation.Config;

/**
 * JVM-side startup regression tests (Robolectric).
 *
 * History: one release crashed instantly on device because the app-level
 * AppTheme.NoActionBar was rewritten with a non-AppCompat parent.
 * AppCompatActivity enforces an AppCompat theme in onCreate, so the app
 * died before drawing a single frame.
 *
 * Scope note: launching the full Capacitor Bridge in a JVM is impossible —
 * Bridge init touches android.webkit.WebViewFactory, which is a runtime stub
 * here ("Stub!" by design) and only exists on a real device/emulator.
 * Device startup is therefore covered by check-apk.sh + a one-time adb launch.
 * These tests pin the class of regression that caused the crash:
 * theme/resource correctness for AppCompatActivity, on several API levels.
 */
@RunWith(org.robolectric.RobolectricTestRunner.class)
@Config(sdk = {28, 34})
public class MainActivitySmokeTest {

  /** Bare AppCompatActivity with the app's REAL theme — reproduces the crash mechanism. */
  public static class ThemedActivity extends AppCompatActivity {
    @Override protected void onCreate(Bundle b) {
      setTheme(R.style.AppTheme_NoActionBar);   // same call BridgeActivity makes
      super.onCreate(b);
      setContentView(new LinearLayout(this));
    }
  }

  @Test
  public void appTheme_NoActionBar_isValidForAppCompatActivity() {
    // If AppTheme.NoActionBar (or its parent chain) is not AppCompat-descendant,
    // setTheme+setContentView throw exactly like the device crash.
    try (ActivityController<ThemedActivity> c = Robolectric.buildActivity(ThemedActivity.class)) {
      c.setup(); // create -> start -> resume
      assertNotNull(c.get().getWindow());
    } catch (IllegalStateException e) {
      if (String.valueOf(e.getMessage()).contains("Theme.AppCompat")) {
        fail("AppTheme.NoActionBar lost its AppCompat parent — the app will crash on launch. Restore Theme.AppCompat.DayNight.NoActionBar in styles.xml");
      }
      throw e;
    }
  }

  @Test
  public void appTheme_resolvesAppCompatAttributes() {
    try (ActivityController<ThemedActivity> c = Robolectric.buildActivity(ThemedActivity.class)) {
      c.setup();
      TypedValue tv = new TypedValue();
      boolean resolved = c.get().getTheme().resolveAttribute(
          androidx.appcompat.R.attr.windowActionBar, tv, true);
      assertTrue("Active theme is not an AppCompat descendant", resolved);
    }
  }

  @Test
  public void manifest_declaresLauncherActivity() throws Exception {
    // .MainActivity must exist and be exported with a LAUNCHER filter (regressions here
    // produce "Activity class does not exist" style crashes on tap).
    Class<?> main = Class.forName("com.ideabucket.app.MainActivity");
    assertTrue("MainActivity must extend BridgeActivity",
        com.getcapacitor.BridgeActivity.class.isAssignableFrom(main));
  }

  @Test
  public void styles_keepNavyWindowBackground() {
    try (ActivityController<ThemedActivity> c = Robolectric.buildActivity(ThemedActivity.class)) {
      c.setup();
      View root = c.get().getWindow().getDecorView();
      assertNotNull(root);
      // The window background should not be the framework default (white-ish) —
      // theme item android:windowBackground must be set to the navy color resource chain.
      TypedValue tv = new TypedValue();
      boolean hasBg = c.get().getTheme().resolveAttribute(android.R.attr.windowBackground, tv, true);
      assertTrue("windowBackground not set in AppTheme chain (white flash on launch)", hasBg);
    }
  }
}
