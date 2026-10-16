<?xml version="1.0" encoding="utf-8"?>
<FrameLayout xmlns:android="http://schemas.android.com/apk/res/android"
    android:layout_width="match_parent"
    android:layout_height="match_parent">

    <!-- ============================================== -->
    <!-- ОСНОВНОЙ UI (панель управления Vesta)          -->
    <!-- ============================================== -->
    <LinearLayout
        android:id="@+id/uiPanel"
        android:layout_width="match_parent"
        android:layout_height="match_parent"
        android:orientation="horizontal"
        android:padding="24dp"
        android:baselineAligned="false">

        <!-- ЛЕВАЯ ЧАСТЬ: путь + кнопка -->
        <LinearLayout
            android:layout_width="0dp"
            android:layout_height="match_parent"
            android:layout_weight="1"
            android:orientation="vertical"
            android:gravity="center"
            android:paddingEnd="16dp">

            <TextView
                android:id="@+id/statusText"
                android:layout_width="wrap_content"
                android:layout_height="wrap_content"
                android:text="Android/media/com.skipp.vesta/game/"
                android:textSize="20sp"
                android:textStyle="bold"
                android:fontFamily="monospace"
                android:textColor="#C62828" />

            <Button
                android:id="@+id/actionButton"
                android:layout_width="wrap_content"
                android:layout_height="wrap_content"
                android:layout_marginTop="24dp"
                android:minWidth="180dp"
                android:text="Import game" />

            <TextView
                android:id="@+id/noDataText"
                android:layout_width="wrap_content"
                android:layout_height="wrap_content"
                android:layout_marginTop="24dp"
                android:text="No game data found"
                android:textSize="16sp"
                android:textColor="#C62828"
                android:visibility="gone" />

            <TextView
                android:id="@+id/detailsText"
                android:layout_width="match_parent"
                android:layout_height="wrap_content"
                android:layout_marginTop="16dp"
                android:textSize="11sp"
                android:fontFamily="monospace"
                android:textColor="#666666" />
        </LinearLayout>

        <!-- ПРАВАЯ ЧАСТЬ: НАСТРОЙКИ -->
        <LinearLayout
            android:layout_width="0dp"
            android:layout_height="match_parent"
            android:layout_weight="1"
            android:orientation="vertical"
            android:paddingStart="16dp">

            <!-- START MODE -->
            <TextView
                android:layout_width="wrap_content"
                android:layout_height="wrap_content"
                android:text="Start Mode"
                android:textSize="16sp"
                android:textStyle="bold" />

            <RadioGroup
                android:id="@+id/startModeGroup"
                android:layout_width="wrap_content"
                android:layout_height="wrap_content"
                android:orientation="horizontal"
                android:layout_marginTop="4dp">

                <RadioButton
                    android:id="@+id/modeStory"
                    android:layout_width="wrap_content"
                    android:layout_height="wrap_content"
                    android:text="Story Mode"
                    android:checked="true" />

                <RadioButton
                    android:id="@+id/modeSandbox"
                    android:layout_width="wrap_content"
                    android:layout_height="wrap_content"
                    android:text="Sandbox Mode"
                    android:layout_marginStart="12dp" />
            </RadioGroup>

            <View
                android:layout_width="match_parent"
                android:layout_height="1dp"
                android:background="#DDDDDD"
                android:layout_marginTop="12dp"
                android:layout_marginBottom="12dp" />

            <!-- FPS LIMIT -->
            <TextView
                android:layout_width="wrap_content"
                android:layout_height="wrap_content"
                android:text="FPS Limit"
                android:textSize="16sp"
                android:textStyle="bold" />

            <RadioGroup
                android:id="@+id/fpsGroup"
                android:layout_width="wrap_content"
                android:layout_height="wrap_content"
                android:orientation="horizontal"
                android:layout_marginTop="4dp">

                <RadioButton android:id="@+id/fps30" android:layout_width="wrap_content" android:layout_height="wrap_content" android:text="30" />
                <RadioButton android:id="@+id/fps40" android:layout_width="wrap_content" android:layout_height="wrap_content" android:text="40" />
                <RadioButton android:id="@+id/fps50" android:layout_width="wrap_content" android:layout_height="wrap_content" android:text="50" />
                <RadioButton android:id="@+id/fps60" android:layout_width="wrap_content" android:layout_height="wrap_content" android:text="60" android:checked="true" />
                <RadioButton android:id="@+id/fpsNoLimit" android:layout_width="wrap_content" android:layout_height="wrap_content" android:text="No limit" />
            </RadioGroup>

            <View
                android:layout_width="match_parent"
                android:layout_height="1dp"
                android:background="#DDDDDD"
                android:layout_marginTop="12dp"
                android:layout_marginBottom="12dp" />

            <!-- TOUCH CONTROLS -->
            <LinearLayout
                android:layout_width="match_parent"
                android:layout_height="wrap_content"
                android:orientation="horizontal"
                android:gravity="center_vertical">

                <TextView
                    android:layout_width="0dp"
                    android:layout_height="wrap_content"
                    android:layout_weight="1"
                    android:text="Touch Controls"
                    android:textSize="16sp"
                    android:textStyle="bold" />

                <Switch
                    android:id="@+id/touchControlsSwitch"
                    android:layout_width="wrap_content"
                    android:layout_height="wrap_content" />
            </LinearLayout>

            <View
                android:layout_width="match_parent"
                android:layout_height="1dp"
                android:background="#DDDDDD"
                android:layout_marginTop="12dp"
                android:layout_marginBottom="12dp" />

            <!-- SHADOW QUALITY -->
            <TextView
                android:layout_width="wrap_content"
                android:layout_height="wrap_content"
                android:text="Shadow Quality"
                android:textSize="16sp"
                android:textStyle="bold" />

            <RadioGroup
                android:id="@+id/shadowGroup"
                android:layout_width="wrap_content"
                android:layout_height="wrap_content"
                android:orientation="horizontal"
                android:layout_marginTop="4dp">

                <RadioButton android:id="@+id/shadow0" android:layout_width="wrap_content" android:layout_height="wrap_content" android:text="0" />
                <RadioButton android:id="@+id/shadow1" android:layout_width="wrap_content" android:layout_height="wrap_content" android:text="1" android:checked="true" />
                <RadioButton android:id="@+id/shadow2" android:layout_width="wrap_content" android:layout_height="wrap_content" android:text="2" />
            </RadioGroup>
        </LinearLayout>
    </LinearLayout>

    <!-- ============================================== -->
    <!-- SURFACEVIEW для рендера wgpu                    -->
    <!-- Скрыт, показывается при нажатии Run             -->
    <!-- ============================================== -->
    <com.skipp.vesta.GameSurfaceView
        android:id="@+id/gameSurface"
        android:layout_width="match_parent"
        android:layout_height="match_parent"
        android:visibility="gone" />

</FrameLayout>