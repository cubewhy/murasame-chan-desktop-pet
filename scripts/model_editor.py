import os
import sys
import json
import shutil
import zipfile
from pathlib import Path
from qtpy import QtWidgets, QtCore, QtGui
from PIL import Image  # Pillow library for image processing


# -----------------------------------------------------------------------------
# Custom Components
# -----------------------------------------------------------------------------
class ZoomableView(QtWidgets.QGraphicsView):
    """
    A QGraphicsView that supports zooming with Ctrl + Mouse Wheel and panning.
    """

    def __init__(self, scene, parent=None):
        super().__init__(scene, parent)
        self.setTransformationAnchor(QtWidgets.QGraphicsView.AnchorUnderMouse)
        self.setResizeAnchor(QtWidgets.QGraphicsView.AnchorUnderMouse)
        self.setDragMode(QtWidgets.QGraphicsView.ScrollHandDrag)  # For panning

    def wheelEvent(self, event: QtGui.QWheelEvent):
        """
        Handles the mouse wheel event for zooming.
        Zooms the view if the Ctrl key is pressed.
        """
        if event.modifiers() == QtCore.Qt.ControlModifier:
            zoom_in_factor = 1.15
            zoom_out_factor = 1 / zoom_in_factor

            # Get the direction of the scroll
            if event.angleDelta().y() > 0:
                self.scale(zoom_in_factor, zoom_in_factor)
            else:
                self.scale(zoom_out_factor, zoom_out_factor)
            event.accept()
        else:
            # Allow default behavior (e.g., vertical scroll) if Ctrl is not pressed
            super().wheelEvent(event)


class ResizablePixmapItem(QtWidgets.QGraphicsPixmapItem):
    """
    A resizable and movable QGraphicsPixmapItem that uses a callback to notify
    of geometry changes (position, size) upon user action completion.
    This is used for the top_layer preview.
    """

    handleSize = 10.0
    handleSpace = -5.0

    def __init__(self, pixmap, parent=None, change_finished_callback=None):
        super().__init__(pixmap, parent)
        self._pixmap = (
            pixmap  # This holds the reference to the ORIGINAL, UNSCALED pixmap
        )
        self.handles = {}
        self.handleSelected = None
        self.mousePressPos = None
        self.mousePressRect = None
        self.change_finished_callback = change_finished_callback
        self.mousePressScenePos = None

        self.setAcceptHoverEvents(True)
        self.setFlag(QtWidgets.QGraphicsItem.ItemIsMovable, True)
        self.setFlag(QtWidgets.QGraphicsItem.ItemIsSelectable, True)
        self.setFlag(QtWidgets.QGraphicsItem.ItemSendsGeometryChanges, True)
        self.setFlag(QtWidgets.QGraphicsItem.ItemIsFocusable, True)
        self.updateHandlesPos()

    def setOriginalPixmap(self, pixmap):
        """
        Sets the base, unscaled pixmap for this item and updates the display.
        This is the main fix for the scaling bug.
        """
        self._pixmap = pixmap
        # Update the currently displayed pixmap to this new original one.
        # Subsequent scaling will be based on this correct _pixmap.
        self.setPixmap(pixmap if pixmap else QtGui.QPixmap())

    def setPixmap(self, pixmap):
        """
        Overrides the base setPixmap to only update the *displayed* pixmap
        and its handles. It does NOT touch the original _pixmap reference.
        """
        super().setPixmap(pixmap)
        self.updateHandlesPos()

    def originalPixmap(self):
        """Returns the original, unscaled pixmap."""
        return self._pixmap

    def mousePressEvent(self, mouseEvent):
        """Detects if a resize handle was clicked or a drag is initiated."""
        super().mousePressEvent(mouseEvent)
        self.handleSelected = self.handleAt(mouseEvent.pos())
        if self.handleSelected:
            self.mousePressPos = mouseEvent.pos()
            self.mousePressRect = self.boundingRect()
        else:
            self.mousePressScenePos = self.scenePos()

    def mouseReleaseEvent(self, mouseEvent):
        """On mouse release after a resize or move, trigger the callback."""
        super().mouseReleaseEvent(mouseEvent)

        if self.handleSelected:
            if self.change_finished_callback:
                self.change_finished_callback(self)

        elif (
            self.mousePressScenePos is not None
            and self.scenePos() != self.mousePressScenePos
        ):
            if self.change_finished_callback:
                self.change_finished_callback(self)

        self.handleSelected = None
        self.mousePressPos = None
        self.mousePressRect = None
        self.mousePressScenePos = None
        self.update()

    def handleAt(self, point):
        """Checks if a given point is inside one of the resize handles."""
        for k, v in self.handles.items():
            if v.contains(point):
                return k
        return None

    def hoverMoveEvent(self, moveEvent):
        """Updates the cursor icon based on which resize handle is hovered."""
        if self.isSelected():
            handle = self.handleAt(moveEvent.pos())
            cursor = (
                self.get_cursor_for_handle(handle) if handle else QtCore.Qt.ArrowCursor
            )
            self.setCursor(cursor)
        super().hoverMoveEvent(moveEvent)

    def get_cursor_for_handle(self, handle):
        """Returns the appropriate QCursor for a given handle position."""
        if handle in (QtCore.Qt.TopLeftCorner, QtCore.Qt.BottomRightCorner):
            return QtCore.Qt.SizeFDiagCursor
        if handle in (QtCore.Qt.TopRightCorner, QtCore.Qt.BottomLeftCorner):
            return QtCore.Qt.SizeBDiagCursor
        if handle in (QtCore.Qt.TopEdge, QtCore.Qt.BottomEdge):
            return QtCore.Qt.SizeVerCursor
        return QtCore.Qt.SizeHorCursor

    def mouseMoveEvent(self, mouseEvent):
        """Handles either resizing or moving the item."""
        if self.handleSelected is not None:
            self.interactiveResize(mouseEvent.pos())
        else:
            super().mouseMoveEvent(mouseEvent)

    def boundingRect(self):
        """Returns the bounding rectangle, including extra space for handles."""
        o = self.handleSize + self.handleSpace
        return self.pixmap().rect().adjusted(-o, -o, o, o)

    def updateHandlesPos(self):
        """Recalculates the positions of the resize handles."""
        s = self.handleSize
        r = self.pixmap().rect()
        self.handles[QtCore.Qt.TopLeftCorner] = QtCore.QRectF(r.left(), r.top(), s, s)
        self.handles[QtCore.Qt.TopRightCorner] = QtCore.QRectF(
            r.right() - s, r.top(), s, s
        )
        self.handles[QtCore.Qt.BottomLeftCorner] = QtCore.QRectF(
            r.left(), r.bottom() - s, s, s
        )
        self.handles[QtCore.Qt.BottomRightCorner] = QtCore.QRectF(
            r.right() - s, r.bottom() - s, s, s
        )

    def interactiveResize(self, mousePos):
        """Handles the logic for resizing the pixmap based on mouse movement."""
        self.prepareGeometryChange()

        if not self._pixmap or self._pixmap.isNull():
            return

        new_rect = QtCore.QRectF(self.pos(), self.pixmap().size().toSizeF())
        if self.handleSelected == QtCore.Qt.BottomRightCorner:
            new_rect.setBottomRight(self.mapToScene(mousePos))

        new_rect = new_rect.normalized()
        self.setPos(new_rect.topLeft())

        new_pixmap = self._pixmap.scaled(
            new_rect.size().toSize(),
            QtCore.Qt.KeepAspectRatio,
            QtCore.Qt.SmoothTransformation,
        )
        self.setPixmap(new_pixmap)

    def paint(self, painter, option, widget=None):
        """Paints the pixmap and, if selected, its border and handles."""
        painter.drawPixmap(QtCore.QPointF(0, 0), self.pixmap())
        if self.isSelected():
            painter.setRenderHint(QtGui.QPainter.Antialiasing)
            painter.setBrush(QtCore.Qt.NoBrush)
            painter.setPen(
                QtGui.QPen(QtGui.QColor(0, 120, 215, 200), 2.0, QtCore.Qt.DashLine)
            )
            painter.drawRect(self.pixmap().rect())
            painter.setPen(QtGui.QPen(QtGui.QColor(0, 120, 215), 1.0))
            painter.setBrush(QtGui.QBrush(QtCore.Qt.white))
            for handle, rect in self.handles.items():
                painter.drawEllipse(rect)


class BindLayersDialog(QtWidgets.QDialog):
    """A custom dialog to select multiple layers to bind."""

    def __init__(self, available_layers, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Bind Layers")
        self.setMinimumWidth(350)
        layout = QtWidgets.QVBoxLayout(self)

        layout.addWidget(QtWidgets.QLabel("Select layer(s) to bind:"))
        self.list_widget = QtWidgets.QListWidget()
        self.list_widget.setSelectionMode(QtWidgets.QAbstractItemView.ExtendedSelection)
        self.list_widget.addItems(available_layers)
        layout.addWidget(self.list_widget)

        buttons = QtWidgets.QDialogButtonBox(
            QtWidgets.QDialogButtonBox.Ok | QtWidgets.QDialogButtonBox.Cancel,
            QtCore.Qt.Horizontal,
            self,
        )
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

    def selected_layers(self):
        """Returns a list of the selected layer names."""
        return [item.text() for item in self.list_widget.selectedItems()]


# -----------------------------------------------------------------------------
# Main Application Window
# -----------------------------------------------------------------------------
class MainWindow(QtWidgets.QMainWindow):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Model Generation Tool")
        self.setGeometry(100, 100, 1400, 900)
        self.project_path = None
        self.manifest = {}
        self.is_dirty = False

        # NEW: List to hold graphics items for bound layers in the preview
        self.bound_preview_items = []

        self.setup_ui()
        self.setup_preview_scene()
        self.create_actions()
        self.create_menus()
        self.create_toolbar()
        self.connect_signals()
        self.setup_auto_save_timer()
        self.update_ui_state()

    def setup_ui(self):
        """Initializes all UI widgets and layouts."""
        # --- Preview Panel (Central Widget) ---
        preview_container = QtWidgets.QWidget()
        preview_layout = QtWidgets.QVBoxLayout(preview_container)
        controls_layout = QtWidgets.QHBoxLayout()
        self.preview_base_combo = QtWidgets.QComboBox(self)
        self.preview_top_combo = QtWidgets.QComboBox(self)

        for combo in [self.preview_base_combo, self.preview_top_combo]:
            combo.setSizePolicy(
                QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Fixed
            )
            combo.setSizeAdjustPolicy(QtWidgets.QComboBox.AdjustToContents)

        self.lock_base_preview_checkbox = QtWidgets.QCheckBox("Lock Base Layer")
        self.lock_base_preview_checkbox.setToolTip(
            "Prevents changing the preview's base layer by selecting layers in the list."
        )

        controls_layout.addWidget(QtWidgets.QLabel("Preview Base Layer:"))
        controls_layout.addWidget(self.preview_base_combo)
        controls_layout.addWidget(QtWidgets.QLabel("Preview Top Layer:"))
        controls_layout.addWidget(self.preview_top_combo)
        controls_layout.addStretch()
        controls_layout.addWidget(self.lock_base_preview_checkbox)

        preview_layout.addLayout(controls_layout)
        self.scene = QtWidgets.QGraphicsScene(self)
        self.view = ZoomableView(self.scene)
        self.view.setRenderHint(QtGui.QPainter.Antialiasing)
        preview_layout.addWidget(self.view)
        self.setCentralWidget(preview_container)

        # --- Left Dock: Project Layers ---
        left_dock = QtWidgets.QDockWidget("Project Layers", self)
        self.addDockWidget(QtCore.Qt.LeftDockWidgetArea, left_dock)
        self.layer_list = QtWidgets.QListWidget()
        left_dock.setWidget(self.layer_list)

        # --- Right Dock: Properties ---
        right_dock = QtWidgets.QDockWidget("Properties", self)
        self.addDockWidget(QtCore.Qt.RightDockWidgetArea, right_dock)
        props_scroll = QtWidgets.QScrollArea()
        props_scroll.setWidgetResizable(True)
        right_dock.setWidget(props_scroll)
        props_widget = QtWidgets.QWidget()
        props_scroll.setWidget(props_widget)
        self.props_layout = QtWidgets.QFormLayout(props_widget)

        # --- Properties Widgets ---
        self.add_layer_button = QtWidgets.QPushButton("Add Layer(s)...")
        self.add_base_layer_button = QtWidgets.QPushButton("Add Base Layer(s)...")
        self.remove_layer_button = QtWidgets.QPushButton("Remove Selected Layer")

        self.set_metadata_file_button = QtWidgets.QPushButton("Assign Metadata File...")
        self.create_metadata_button = QtWidgets.QPushButton("Create New Metadata")

        self.import_metadata_button = QtWidgets.QPushButton(
            "Import Matching Metadata..."
        )
        self.filter_checkbox = QtWidgets.QCheckBox("Hide layers with assigned metadata")

        self.layer_name_label = QtWidgets.QLabel("<No layer selected>")
        self.description_edit = QtWidgets.QLineEdit()
        self.metadata_label = QtWidgets.QLabel("None")
        self.clear_metadata_button = QtWidgets.QPushButton("Clear")
        self.is_base_checkbox = QtWidgets.QCheckBox("Is Base Layer")
        self.offset_x_spinbox = QtWidgets.QSpinBox()
        self.offset_y_spinbox = QtWidgets.QSpinBox()
        for spinbox in [self.offset_x_spinbox, self.offset_y_spinbox]:
            spinbox.setRange(-10000, 10000)

        self.metadata_x_spinbox = QtWidgets.QSpinBox()
        self.metadata_y_spinbox = QtWidgets.QSpinBox()
        self.metadata_scale_spinbox = QtWidgets.QDoubleSpinBox()
        for spinbox in [self.metadata_x_spinbox, self.metadata_y_spinbox]:
            spinbox.setRange(-10000, 10000)
        self.metadata_scale_spinbox.setRange(0.01, 100.0)
        self.metadata_scale_spinbox.setSingleStep(0.01)
        self.metadata_scale_spinbox.setDecimals(3)

        self.metadata_opacity_spinbox = QtWidgets.QDoubleSpinBox()
        self.metadata_opacity_spinbox.setRange(0.0, 1.0)
        self.metadata_opacity_spinbox.setSingleStep(0.05)
        self.metadata_opacity_spinbox.setDecimals(2)
        self.metadata_opacity_spinbox.setValue(1.0)

        offset_widget = QtWidgets.QWidget()
        offset_layout = QtWidgets.QHBoxLayout(offset_widget)
        offset_layout.setContentsMargins(0, 0, 0, 0)
        offset_layout.addWidget(QtWidgets.QLabel("X:"))
        offset_layout.addWidget(self.offset_x_spinbox)
        offset_layout.addWidget(QtWidgets.QLabel("Y:"))
        offset_layout.addWidget(self.offset_y_spinbox)
        self.offset_widget = offset_widget

        metadata_props_widget = QtWidgets.QWidget()
        metadata_props_layout = QtWidgets.QHBoxLayout(metadata_props_widget)
        metadata_props_layout.setContentsMargins(0, 0, 0, 0)
        metadata_props_layout.addWidget(QtWidgets.QLabel("X:"))
        metadata_props_layout.addWidget(self.metadata_x_spinbox)
        metadata_props_layout.addWidget(QtWidgets.QLabel("Y:"))
        metadata_props_layout.addWidget(self.metadata_y_spinbox)
        metadata_props_layout.addWidget(QtWidgets.QLabel("Scale:"))
        metadata_props_layout.addWidget(self.metadata_scale_spinbox)
        metadata_props_layout.addWidget(QtWidgets.QLabel("Opacity:"))
        metadata_props_layout.addWidget(self.metadata_opacity_spinbox)
        self.metadata_props_widget = metadata_props_widget

        metadata_widget = QtWidgets.QWidget()
        metadata_layout = QtWidgets.QHBoxLayout(metadata_widget)
        metadata_layout.setContentsMargins(0, 0, 0, 0)
        metadata_layout.addWidget(self.set_metadata_file_button)
        metadata_layout.addWidget(self.create_metadata_button)
        metadata_layout.addWidget(self.clear_metadata_button)

        # NEW: Layer Binding UI
        self.bound_layers_list = QtWidgets.QListWidget()
        self.bound_layers_list.setSelectionMode(
            QtWidgets.QAbstractItemView.ExtendedSelection
        )
        self.bound_layers_list.setFixedHeight(120)

        bind_buttons_widget = QtWidgets.QWidget()
        bind_buttons_layout = QtWidgets.QHBoxLayout(bind_buttons_widget)
        bind_buttons_layout.setContentsMargins(0, 0, 0, 0)
        self.bind_layer_button = QtWidgets.QPushButton("Bind Layer(s)...")
        self.unbind_layer_button = QtWidgets.QPushButton("Unbind Selected")
        bind_buttons_layout.addWidget(self.bind_layer_button)
        bind_buttons_layout.addWidget(self.unbind_layer_button)

        self.props_layout.addRow(self.add_layer_button)
        self.props_layout.addRow(self.add_base_layer_button)
        self.props_layout.addRow(self.remove_layer_button)
        self.props_layout.addRow(self.import_metadata_button)

        separator = QtWidgets.QFrame()
        separator.setFrameShape(QtWidgets.QFrame.HLine)
        separator.setFrameShadow(QtWidgets.QFrame.Sunken)
        self.props_layout.addRow(separator)
        self.props_layout.addRow("Filter:", self.filter_checkbox)

        self.props_layout.addRow(QtWidgets.QLabel("--- Selected Layer ---"))
        self.props_layout.addRow("Layer:", self.layer_name_label)
        self.props_layout.addRow("Description:", self.description_edit)
        self.props_layout.addRow("Metadata File:", self.metadata_label)
        self.props_layout.addRow("", metadata_widget)
        self.props_layout.addRow(self.is_base_checkbox)
        self.props_layout.addRow("Base Offset:", self.offset_widget)
        self.props_layout.addRow("Metadata Props:", self.metadata_props_widget)

        # NEW: Add binding widgets to layout
        self.props_layout.addRow(QtWidgets.QLabel("--- Layer Bindings ---"))
        self.props_layout.addRow(self.bound_layers_list)
        self.props_layout.addRow(bind_buttons_widget)

        # --- Bottom Dock: Metadata Editor ---
        editor_dock = QtWidgets.QDockWidget("Metadata Editor", self)
        self.addDockWidget(QtCore.Qt.BottomDockWidgetArea, editor_dock)
        editor_container = QtWidgets.QWidget()
        editor_layout = QtWidgets.QVBoxLayout(editor_container)
        self.metadata_editor = QtWidgets.QTextEdit()
        self.metadata_editor.setFont(QtGui.QFont("Courier", 10))
        self.metadata_editor.setPlaceholderText(
            "Select a layer with metadata to edit its content here..."
        )
        self.save_metadata_button = QtWidgets.QPushButton("Save Metadata Changes")
        editor_layout.addWidget(self.metadata_editor)
        editor_layout.addWidget(self.save_metadata_button)
        editor_dock.setWidget(editor_container)

    def setup_preview_scene(self):
        """Initializes the QGraphicsScene and its items for the preview."""
        self.preview_base_item = QtWidgets.QGraphicsPixmapItem()
        self.preview_top_item = ResizablePixmapItem(
            QtGui.QPixmap(), change_finished_callback=self.handle_preview_item_change
        )
        self.scene.addItem(self.preview_base_item)
        self.scene.addItem(self.preview_top_item)
        self.preview_base_item.setVisible(False)
        self.preview_top_item.setVisible(False)

    def create_actions(self):
        """Creates QAction objects for menus and toolbars."""
        self.new_action = QtGui.QAction("New Project...", self)
        self.open_action = QtGui.QAction("Open Project...", self)
        self.save_action = QtGui.QAction("Save Project", self)
        self.save_as_zip_action = QtGui.QAction("Export to model.zip...", self)
        self.import_metadata_action = QtGui.QAction("Import Matching Metadata...", self)
        self.quit_action = QtGui.QAction("Quit", self)

        # --- Actions for Tools Menu ---
        self.migrate_action = QtGui.QAction("Migrate Project Metadata...", self)
        self.cleanup_manifest_action = QtGui.QAction("Clean Up Manifest...", self)

    def create_menus(self):
        """Creates the main menu bar."""
        menu_bar = self.menuBar()
        file_menu = menu_bar.addMenu("File")
        file_menu.addAction(self.new_action)
        file_menu.addAction(self.open_action)
        file_menu.addAction(self.save_action)
        file_menu.addSeparator()
        file_menu.addAction(self.import_metadata_action)
        file_menu.addSeparator()
        file_menu.addAction(self.save_as_zip_action)
        file_menu.addSeparator()
        file_menu.addAction(self.quit_action)

        # --- New Tools Menu ---
        tools_menu = menu_bar.addMenu("Tools")
        tools_menu.addAction(self.migrate_action)
        tools_menu.addAction(self.cleanup_manifest_action)

    def create_toolbar(self):
        """Creates the main toolbar."""
        toolbar = self.addToolBar("Main")
        toolbar.addAction(self.new_action)
        toolbar.addAction(self.open_action)
        toolbar.addAction(self.save_action)

    def connect_signals(self):
        """Connects widget signals to their corresponding handler slots."""
        self.new_action.triggered.connect(self.new_project)
        self.open_action.triggered.connect(self.open_project)
        self.save_action.triggered.connect(self.save_project)
        self.save_as_zip_action.triggered.connect(self.save_as_zip)
        self.import_metadata_action.triggered.connect(self.import_metadata)
        self.quit_action.triggered.connect(self.close)

        # --- Connect Tool Actions ---
        self.migrate_action.triggered.connect(self.run_migration_script)
        self.cleanup_manifest_action.triggered.connect(self.cleanup_manifest)

        self.layer_list.currentItemChanged.connect(self.on_layer_selection_changed)
        self.add_layer_button.clicked.connect(self.add_layers)
        self.add_base_layer_button.clicked.connect(self.add_base_layers)
        self.remove_layer_button.clicked.connect(self.remove_layer)
        self.import_metadata_button.clicked.connect(self.import_metadata)
        self.set_metadata_file_button.clicked.connect(self.set_metadata_file)
        self.create_metadata_button.clicked.connect(self.create_metadata_file)
        self.clear_metadata_button.clicked.connect(self.clear_metadata)
        self.filter_checkbox.stateChanged.connect(self.populate_layer_list)

        self.preview_base_combo.currentIndexChanged.connect(
            self.on_base_preview_changed
        )
        self.preview_top_combo.currentIndexChanged.connect(self.on_top_preview_changed)

        self.save_metadata_button.clicked.connect(self.save_metadata_from_editor)

        self.is_base_checkbox.stateChanged.connect(self.update_layer_type)
        self.description_edit.textChanged.connect(self.update_layer_description)
        self.offset_x_spinbox.valueChanged.connect(self.update_base_offset)
        self.offset_y_spinbox.valueChanged.connect(self.update_base_offset)
        self.metadata_x_spinbox.valueChanged.connect(
            self.update_metadata_from_spinboxes
        )
        self.metadata_y_spinbox.valueChanged.connect(
            self.update_metadata_from_spinboxes
        )
        self.metadata_scale_spinbox.valueChanged.connect(
            self.update_metadata_from_spinboxes
        )
        self.metadata_opacity_spinbox.valueChanged.connect(
            self.update_metadata_from_spinboxes
        )

        # NEW: Connect binding button signals
        self.bind_layer_button.clicked.connect(self.bind_layers)
        self.unbind_layer_button.clicked.connect(self.unbind_layers)
        self.bound_layers_list.itemSelectionChanged.connect(self.update_unbind_button_state)

    def new_project(self):
        """Handles the creation of a new project in an empty directory."""
        if not self.prompt_save_if_dirty():
            return
        path_str = QtWidgets.QFileDialog.getExistingDirectory(
            self, "Select an Empty Directory for New Project"
        )
        if not path_str:
            return
        if len(os.listdir(path_str)) > 0:
            QtWidgets.QMessageBox.warning(
                self, "Warning", "Please select an empty directory."
            )
            return
        self.project_path = Path(path_str)
        (self.project_path / "layers").mkdir(exist_ok=True)
        (self.project_path / "metadata").mkdir(exist_ok=True)
        self.manifest = {"layers": {}}
        self.save_manifest()
        self.load_project(self.project_path)
        self.statusBar().showMessage(f"New project created at {path_str}", 5000)

    def open_project(self):
        """Opens an existing project from a directory."""
        if not self.prompt_save_if_dirty():
            return
        path_str = QtWidgets.QFileDialog.getExistingDirectory(
            self, "Select Project Directory"
        )
        if path_str:
            self.load_project(Path(path_str))

    def save_project(self):
        """Saves the current project's manifest file."""
        if not self.project_path:
            QtWidgets.QMessageBox.warning(self, "Error", "No project is open.")
            return False
        self.save_manifest()
        self.is_dirty = False
        self._update_window_title()
        self.statusBar().showMessage("Project saved.", 3000)
        return True

    def save_as_zip(self):
        """Exports the entire project directory to a single .zip file."""
        if not self.project_path:
            QtWidgets.QMessageBox.warning(self, "Error", "No project is open.")
            return
        self.save_project()  # Ensure manifest is up-to-date before zipping
        path, _ = QtWidgets.QFileDialog.getSaveFileName(
            self, "Export to model.zip", "", "ZIP Files (*.zip)"
        )
        if not path:
            return
        with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as zipf:
            for root, _, files in os.walk(self.project_path):
                for file in files:
                    file_path = os.path.join(root, file)
                    archive_name = os.path.relpath(file_path, self.project_path)
                    zipf.write(file_path, archive_name)
        QtWidgets.QMessageBox.information(
            self, "Success", f"Project exported to {path}"
        )

    def load_project(self, path: Path):
        """Loads project data from a specified path."""
        manifest_path = path / "manifest.json"
        if not manifest_path.exists() or not (path / "layers").is_dir():
            QtWidgets.QMessageBox.critical(
                self, "Error", "'manifest.json' and a 'layers' folder must exist."
            )
            return
        self.project_path = path
        try:
            with open(manifest_path, "r", encoding="utf-8") as f:
                self.manifest = json.load(f)
            self.manifest.setdefault("layers", {})
        except Exception as e:
            QtWidgets.QMessageBox.critical(
                self, "Load Failed", f"Error reading manifest.json: {e}"
            )
            self.project_path = None
            return
        self.is_dirty = False
        self.populate_layer_list()
        self.update_ui_state()
        self._update_window_title()
        self.statusBar().showMessage(
            f"Project '{self.project_path.name}' loaded.", 5000
        )

    def save_manifest(self):
        """Writes the current manifest dictionary to manifest.json."""
        if self.project_path:
            with open(self.project_path / "manifest.json", "w", encoding="utf-8") as f:
                json.dump(self.manifest, f, indent=4)

    def _update_window_title(self):
        """Updates the window title to show project name and dirty status."""
        title = "Model Generation Tool"
        if self.project_path:
            dirty_indicator = "*" if self.is_dirty else ""
            title = f"Model Gen Tool - {self.project_path.name}{dirty_indicator}"
        self.setWindowTitle(title)

    def mark_dirty(self):
        """Flags the project as having unsaved changes."""
        if not self.is_dirty:
            self.is_dirty = True
            self._update_window_title()

    def update_ui_state(self):
        """Enables or disables UI elements based on the application's state."""
        is_project_open = self.project_path is not None
        current_item = self.layer_list.currentItem()
        is_layer_selected = current_item is not None
        is_base_layer_selected = False
        has_metadata = False

        if is_layer_selected:
            layer_name = self.get_layer_name_from_item(current_item)
            if layer_name in self.manifest["layers"]:
                layer_data = self.manifest["layers"][layer_name]
                is_base_layer_selected = layer_data.get("type") == "base_layer"
                has_metadata = "metadata" in layer_data and layer_data["metadata"]

        self.save_action.setEnabled(is_project_open)
        self.save_as_zip_action.setEnabled(is_project_open)
        self.import_metadata_action.setEnabled(is_project_open)
        self.centralWidget().setEnabled(is_project_open)
        self.layer_list.setEnabled(is_project_open)
        self.add_layer_button.setEnabled(is_project_open)
        self.add_base_layer_button.setEnabled(is_project_open)
        self.import_metadata_button.setEnabled(is_project_open)
        self.remove_layer_button.setEnabled(is_layer_selected)
        self.filter_checkbox.setEnabled(is_project_open)

        self.is_base_checkbox.setEnabled(is_layer_selected)
        self.description_edit.setEnabled(is_layer_selected)
        self.offset_widget.setEnabled(is_layer_selected and is_base_layer_selected)

        self.set_metadata_file_button.setEnabled(
            is_layer_selected and not is_base_layer_selected
        )
        self.create_metadata_button.setEnabled(
            is_layer_selected and not is_base_layer_selected and not has_metadata
        )
        self.clear_metadata_button.setEnabled(
            bool(is_layer_selected and not is_base_layer_selected and has_metadata)
        )

        self.metadata_props_widget.setEnabled(
            bool(is_layer_selected and not is_base_layer_selected and has_metadata)
        )

        self.metadata_editor.setEnabled(bool(is_layer_selected and has_metadata))
        self.save_metadata_button.setEnabled(bool(is_layer_selected and has_metadata))

        # NEW: Update binding UI state
        self.bound_layers_list.setEnabled(is_layer_selected)
        self.bind_layer_button.setEnabled(is_layer_selected)
        
        if not is_layer_selected:
            self.unbind_layer_button.setEnabled(False)

        if not is_layer_selected:
            self.layer_name_label.setText("<No layer selected>")
            self.description_edit.clear()
            self.metadata_label.setText("None")
            self.is_base_checkbox.setChecked(False)
            self.offset_x_spinbox.setValue(0)
            self.offset_y_spinbox.setValue(0)
            self.metadata_x_spinbox.setValue(0)
            self.metadata_y_spinbox.setValue(0)
            self.metadata_scale_spinbox.setValue(1.0)
            self.metadata_opacity_spinbox.setValue(1.0)
            self.metadata_editor.clear()
            self.bound_layers_list.clear()  # NEW

    def update_unbind_button_state(self):
        """Enables or disables the 'Unbind' button based on selection in the bindings list."""
        # The button is enabled only if there are items selected in the list.
        has_selection = len(self.bound_layers_list.selectedItems()) > 0
        self.unbind_layer_button.setEnabled(has_selection)

    def get_layer_name_from_item(self, item: QtWidgets.QListWidgetItem) -> str:
        """Extracts the layer name from a QListWidgetItem's text."""
        if not item:
            return ""
        try:
            return item.text().split(" ", 1)[1]
        except (RuntimeError, IndexError):
            return ""

    def get_prefix_for_layer(self, layer_name):
        """Determines the prefix for a layer item based on its state."""
        data = self.manifest["layers"].get(layer_name, {})
        if data.get("type") == "base_layer":
            return "[B]"
        if "metadata" in data and data["metadata"]:
            return "[M]"
        return "[X]"

    def populate_layer_list(self):
        """
        Fills the layer list widget with items from the manifest,
        applying the filter if it is active.
        """
        current_selection_name = self.get_layer_name_from_item(
            self.layer_list.currentItem()
        )

        self.layer_list.blockSignals(True)
        self.layer_list.clear()

        if not self.project_path:
            self.layer_list.blockSignals(False)
            return

        is_filter_active = self.filter_checkbox.isChecked()
        new_row_to_select = -1
        current_row_index = 0
        sorted_layers = sorted(self.manifest.get("layers", {}).keys())

        for name in sorted_layers:
            data = self.manifest["layers"][name]
            has_metadata = "metadata" in data and data["metadata"]

            should_show = not is_filter_active or (
                is_filter_active and not has_metadata
            )

            if should_show:
                prefix = self.get_prefix_for_layer(name)
                item = QtWidgets.QListWidgetItem(f"{prefix} {name}")
                self.layer_list.addItem(item)
                if name == current_selection_name:
                    new_row_to_select = current_row_index
                current_row_index += 1

        self.layer_list.blockSignals(False)

        if new_row_to_select != -1:
            self.layer_list.setCurrentRow(new_row_to_select)
        elif self.layer_list.count() > 0:
            self.layer_list.setCurrentRow(0)

        self.populate_combo_boxes()

    def populate_combo_boxes(self):
        """Fills the preview dropdowns with appropriate layer names."""
        base_text = self.preview_base_combo.currentText()
        top_text = self.preview_top_combo.currentText()

        self.preview_base_combo.blockSignals(True)
        self.preview_top_combo.blockSignals(True)

        self.preview_base_combo.clear()
        self.preview_top_combo.clear()

        layers = self.manifest.get("layers", {})
        base_items = ["<None>"] + sorted(
            [n for n, d in layers.items() if d.get("type") == "base_layer"]
        )
        top_items = ["<None>"] + sorted(
            [n for n, d in layers.items() if d.get("type") != "base_layer"]
        )

        self.preview_base_combo.addItems(base_items)
        self.preview_top_combo.addItems(top_items)

        if base_text in base_items:
            self.preview_base_combo.setCurrentText(base_text)
        if top_text in top_items:
            self.preview_top_combo.setCurrentText(top_text)

        self.preview_base_combo.blockSignals(False)
        self.preview_top_combo.blockSignals(False)

        self.update_preview(fit_view=True)

    def on_layer_selection_changed(self, current, previous):
        """Handles changes in the selected layer in the list."""
        if not current:
            self.update_ui_state()
            return

        layer_name = self.get_layer_name_from_item(current)
        if not layer_name:
            return
        data = self.manifest["layers"].get(layer_name, {})

        # Block signals to prevent feedback loops
        widgets_to_block = [
            self.is_base_checkbox,
            self.description_edit,
            self.offset_x_spinbox,
            self.offset_y_spinbox,
            self.metadata_x_spinbox,
            self.metadata_y_spinbox,
            self.metadata_scale_spinbox,
            self.metadata_opacity_spinbox,
            self.bound_layers_list,
        ]
        for widget in widgets_to_block:
            widget.blockSignals(True)

        self.layer_name_label.setText(layer_name)
        self.description_edit.setText(data.get("description", ""))
        metadata_filename = data.get("metadata", "None")
        self.metadata_label.setText(os.path.basename(metadata_filename))
        is_base = data.get("type") == "base_layer"
        self.is_base_checkbox.setChecked(is_base)
        self.offset_x_spinbox.setValue(data.get("offset", [0, 0])[0])
        self.offset_y_spinbox.setValue(data.get("offset", [0, 0])[1])

        # NEW: Populate bound layers list
        self.bound_layers_list.clear()
        self.bound_layers_list.addItems(data.get("bindings", []))

        self.metadata_editor.clear()
        if metadata_filename != "None":
            metadata_content = self.load_metadata_content_for_layer(layer_name)
            if metadata_content:
                self.metadata_editor.setPlainText(metadata_content)

            metadata_json = self.load_metadata_json_for_layer(layer_name)
            if metadata_json and "top_layer" in metadata_json:
                meta = metadata_json["top_layer"]
                self.metadata_x_spinbox.setValue(meta.get("x", 0))
                self.metadata_y_spinbox.setValue(meta.get("y", 0))
                self.metadata_scale_spinbox.setValue(meta.get("scale", 1.0))
                self.metadata_opacity_spinbox.setValue(meta.get("opacity", 1.0))
            else:
                self.metadata_x_spinbox.setValue(0)
                self.metadata_y_spinbox.setValue(0)
                self.metadata_scale_spinbox.setValue(1.0)
                self.metadata_opacity_spinbox.setValue(1.0)
        else:
            self.metadata_x_spinbox.setValue(0)
            self.metadata_y_spinbox.setValue(0)
            self.metadata_scale_spinbox.setValue(1.0)
            self.metadata_opacity_spinbox.setValue(1.0)

        for widget in widgets_to_block:
            widget.blockSignals(False)

        self.update_ui_state()

        if is_base:
            if not self.lock_base_preview_checkbox.isChecked():
                index = self.preview_base_combo.findText(layer_name)
                if index != -1:
                    self.preview_base_combo.setCurrentIndex(index)
        else:
            index = self.preview_top_combo.findText(layer_name)
            if index != -1:
                self.preview_top_combo.setCurrentIndex(index)

        self.update_unbind_button_state()

    def update_layer_type(self, *args):
        current_item = self.layer_list.currentItem()
        if not current_item:
            return
        layer_name = self.get_layer_name_from_item(current_item)
        data = self.manifest["layers"][layer_name]
        if self.is_base_checkbox.isChecked():
            data["type"] = "base_layer"
            data.setdefault("offset", [0, 0])
            data.pop("metadata", None)
        else:
            data.pop("type", None)
            data.pop("offset", None)
        self.mark_dirty()
        self.populate_layer_list()
        self.update_ui_state()

    def update_base_offset(self, *args):
        current_item = self.layer_list.currentItem()
        if not current_item:
            return
        layer_name = self.get_layer_name_from_item(current_item)
        data = self.manifest["layers"][layer_name]
        if data.get("type") == "base_layer":
            data["offset"] = [
                self.offset_x_spinbox.value(),
                self.offset_y_spinbox.value(),
            ]
            self.mark_dirty()
            self.update_preview()

    def update_layer_description(self, *args):
        current_item = self.layer_list.currentItem()
        if not current_item:
            return
        layer_name = self.get_layer_name_from_item(current_item)
        data = self.manifest["layers"][layer_name]
        data["description"] = self.description_edit.text()
        self.mark_dirty()

    def update_metadata_from_spinboxes(self, *args):
        """Programmatically updates the JSON metadata file from the spinboxes."""
        current_item = self.layer_list.currentItem()
        if not current_item:
            return
        layer_name = self.get_layer_name_from_item(current_item)
        metadata_json = self.load_metadata_json_for_layer(layer_name)
        if not metadata_json:
            return

        top_layer_info = metadata_json.setdefault("top_layer", {})
        top_layer_info["x"] = self.metadata_x_spinbox.value()
        top_layer_info["y"] = self.metadata_y_spinbox.value()
        new_scale = self.metadata_scale_spinbox.value()
        top_layer_info["scale"] = new_scale
        top_layer_info["opacity"] = self.metadata_opacity_spinbox.value()

        original_w = top_layer_info.get("original_width")
        original_h = top_layer_info.get("original_height")

        if original_w is not None and original_h is not None:
            top_layer_info["scaled_width"] = int(original_w * new_scale)
            top_layer_info["scaled_height"] = int(original_h * new_scale)

        metadata_filename = self.manifest["layers"].get(layer_name, {}).get("metadata")
        if not metadata_filename:
            return
        metadata_path = self.project_path / "metadata" / metadata_filename
        try:
            with open(metadata_path, "w", encoding="utf-8") as f:
                json.dump(metadata_json, f, indent=4)
            self.metadata_editor.blockSignals(True)
            self.metadata_editor.setPlainText(json.dumps(metadata_json, indent=4))
            self.metadata_editor.blockSignals(False)
            self.mark_dirty()
            self.update_preview()
        except (IOError, TypeError) as e:
            QtWidgets.QMessageBox.critical(
                self,
                "Save Error",
                f"Could not write metadata to file {metadata_path}.\n\nError: {e}",
            )

    def _add_layer_files(self, paths, is_base=False):
        """Helper function to copy layer files into the project."""
        if not paths:
            return
        for path in paths:
            src, dest = Path(path), self.project_path / "layers" / Path(path).name
            if dest.exists():
                reply = QtWidgets.QMessageBox.question(
                    self,
                    "File Exists",
                    f"'{src.name}' already exists. Overwrite?",
                    QtWidgets.QMessageBox.Yes | QtWidgets.QMessageBox.No,
                )
                if reply == QtWidgets.QMessageBox.No:
                    continue
            shutil.copy(src, dest)
            if is_base:
                self.manifest["layers"][src.name] = {
                    "type": "base_layer",
                    "offset": [0, 0],
                }
            else:
                self.manifest["layers"].setdefault(src.name, {})
        self.mark_dirty()
        self.populate_layer_list()

    def add_layers(self):
        """Opens a dialog to add regular layer images."""
        paths, _ = QtWidgets.QFileDialog.getOpenFileNames(
            self, "Add Layer Image(s)", "", "Image Files (*.png *.jpg *.jpeg)"
        )
        self._add_layer_files(paths, is_base=False)

    def add_base_layers(self):
        """Opens a dialog to add base layer images."""
        paths, _ = QtWidgets.QFileDialog.getOpenFileNames(
            self, "Add Base Layer Image(s)", "", "Image Files (*.png *.jpg *.jpeg)"
        )
        self._add_layer_files(paths, is_base=True)

    def remove_layer(self):
        """Removes the selected layer from the project."""
        current_item = self.layer_list.currentItem()
        if not current_item:
            return
        layer_name = self.get_layer_name_from_item(current_item)
        reply = QtWidgets.QMessageBox.question(
            self,
            "Confirm Deletion",
            f"Delete '{layer_name}' from project?",
            QtWidgets.QMessageBox.Yes | QtWidgets.QMessageBox.No,
        )
        if reply == QtWidgets.QMessageBox.Yes:
            self.manifest["layers"].pop(layer_name, None)
            (self.project_path / "layers" / layer_name).unlink(missing_ok=True)
            self.mark_dirty()
            self.populate_layer_list()
            self.update_ui_state()

    def set_metadata_file(self):
        """Opens a dialog to select a metadata file for the current layer."""
        current_item = self.layer_list.currentItem()
        if not current_item:
            return
        layer_name = self.get_layer_name_from_item(current_item)
        path, _ = QtWidgets.QFileDialog.getOpenFileName(
            self, "Set Metadata File", "", "JSON Files (*.json)"
        )
        if not path:
            return
        src, dest = Path(path), self.project_path / "metadata" / Path(path).name
        shutil.copy(src, dest)
        self.manifest["layers"][layer_name]["metadata"] = dest.name
        self.mark_dirty()
        self.populate_layer_list()
        self.on_layer_selection_changed(self.layer_list.currentItem(), None)

    def _create_and_assign_metadata(self, layer_name):
        """
        Internal function to create a default metadata file for a layer.
        Returns the new filename on success, None on failure.
        """
        image_path = self.project_path / "layers" / layer_name
        original_w, original_h = 0, 0
        if image_path.exists():
            pixmap = QtGui.QPixmap(str(image_path))
            original_w = pixmap.width()
            original_h = pixmap.height()

        default_metadata = {
            "top_layer": {
                "x": 0,
                "y": 0,
                "scale": 1.0,
                "opacity": 1.0,
                "original_width": original_w,
                "original_height": original_h,
                "scaled_width": original_w,
                "scaled_height": original_h,
            }
        }
        metadata_filename = f"{Path(layer_name).stem}.json"
        metadata_path = self.project_path / "metadata" / metadata_filename

        try:
            (self.project_path / "metadata").mkdir(exist_ok=True)
            with open(metadata_path, "w", encoding="utf-8") as f:
                json.dump(default_metadata, f, indent=4)
        except IOError as e:
            print(f"Error creating metadata file: {e}")
            return None

        self.manifest["layers"][layer_name]["metadata"] = metadata_filename
        self.mark_dirty()
        return metadata_filename

    def create_metadata_file(self):
        """Creates a default metadata file for the selected layer."""
        current_item = self.layer_list.currentItem()
        if not current_item:
            return
        layer_name = self.get_layer_name_from_item(current_item)

        metadata_filename = f"{Path(layer_name).stem}.json"
        metadata_path = self.project_path / "metadata" / metadata_filename

        if metadata_path.exists():
            reply = QtWidgets.QMessageBox.question(
                self,
                "File Exists",
                f"'{metadata_filename}' already exists. Overwrite it with default values?",
                QtWidgets.QMessageBox.Yes | QtWidgets.QMessageBox.No,
            )
            if reply == QtWidgets.QMessageBox.No:
                return

        new_filename = self._create_and_assign_metadata(layer_name)
        if new_filename:
            current_item.setText(
                f"{self.get_prefix_for_layer(layer_name)} {layer_name}"
            )
            self.on_layer_selection_changed(current_item, None)
            self.statusBar().showMessage(
                f"Created metadata file '{new_filename}'", 4000
            )
        else:
            QtWidgets.QMessageBox.critical(
                self, "Error", f"Could not create metadata file."
            )

    def clear_metadata(self):
        """Removes the metadata association from the selected layer."""
        current_item = self.layer_list.currentItem()
        if not current_item:
            return
        layer_name = self.get_layer_name_from_item(current_item)
        if "metadata" in self.manifest["layers"][layer_name]:
            self.manifest["layers"][layer_name].pop("metadata")
            self.mark_dirty()
            self.populate_layer_list()
            self.on_layer_selection_changed(self.layer_list.currentItem(), None)

    def import_metadata(self):
        """Batch imports metadata files, matching them to layers by filename."""
        if not self.project_path:
            return
        paths, _ = QtWidgets.QFileDialog.getOpenFileNames(
            self, "Import Metadata Files", "", "JSON Files (*.json)"
        )
        if not paths:
            return
        (self.project_path / "metadata").mkdir(exist_ok=True)
        matched_count = 0
        unmatched_files = []
        layer_basename_map = {Path(name).stem: name for name in self.manifest["layers"]}
        for path in paths:
            json_path = Path(path)
            json_stem = json_path.stem
            if json_stem in layer_basename_map:
                layer_name = layer_basename_map[json_stem]
                dest = self.project_path / "metadata" / json_path.name
                shutil.copy(json_path, dest)
                self.manifest["layers"][layer_name]["metadata"] = json_path.name
                matched_count += 1
            else:
                unmatched_files.append(json_path.name)
        if matched_count > 0:
            self.mark_dirty()
            self.populate_layer_list()
            if self.layer_list.currentItem():
                self.on_layer_selection_changed(self.layer_list.currentItem(), None)
        message = f"Successfully imported metadata for {matched_count} layer(s)."
        if unmatched_files:
            message += f"\n\nCould not find matching layers for:\n- " + "\n- ".join(
                unmatched_files
            )
        QtWidgets.QMessageBox.information(self, "Import Complete", message)

    def setup_auto_save_timer(self):
        """Sets up a timer for periodic auto-saving of the project."""
        self.auto_save_timer = QtCore.QTimer(self)
        self.auto_save_timer.timeout.connect(self.auto_save_project)
        self.auto_save_timer.start(30000)  # 30 seconds

    def auto_save_project(self):
        """Saves the project if it has unsaved changes."""
        if self.project_path and self.is_dirty:
            self.save_project()
            self.statusBar().showMessage("Project auto-saved.", 3000)

    def prompt_save_if_dirty(self) -> bool:
        """Asks the user to save if there are changes. Returns False if cancelled."""
        if not self.is_dirty:
            return True
        reply = QtWidgets.QMessageBox.question(
            self,
            "Unsaved Changes",
            "You have unsaved changes. Do you want to save them?",
            QtWidgets.QMessageBox.Save
            | QtWidgets.QMessageBox.Discard
            | QtWidgets.QMessageBox.Cancel,
            QtWidgets.QMessageBox.Save,
        )
        if reply == QtWidgets.QMessageBox.Save:
            return self.save_project()
        if reply == QtWidgets.QMessageBox.Cancel:
            return False
        return True  # Discard

    def closeEvent(self, event):
        """Handles the window close event, prompting to save if needed."""
        if self.prompt_save_if_dirty():
            event.accept()
        else:
            event.ignore()

    def load_metadata_content_for_layer(self, layer_name):
        """Loads and returns the raw string content of a layer's metadata file."""
        if not layer_name or not self.project_path:
            return None
        metadata_filename = self.manifest["layers"].get(layer_name, {}).get("metadata")
        if not metadata_filename:
            return None

        metadata_path = self.project_path / "metadata" / metadata_filename
        if metadata_path.exists():
            try:
                with open(metadata_path, "r", encoding="utf-8") as f:
                    return f.read()
            except IOError:
                return None
        return None

    def load_metadata_json_for_layer(self, layer_name):
        """Loads, parses, and returns the JSON data from a layer's metadata file."""
        content = self.load_metadata_content_for_layer(layer_name)
        if content:
            try:
                return json.loads(content)
            except json.JSONDecodeError:
                return None
        return None

    def on_base_preview_changed(self, *args):
        """Updates the preview and resets the zoom/pan when the base layer changes."""
        self.update_preview(fit_view=True)

    def on_top_preview_changed(self, *args):
        """Updates the preview without changing zoom/pan when the top layer changes."""
        self.update_preview(fit_view=False)

    def update_preview(self, fit_view=False):
        """Rerenders the preview scene based on current selections, data, and bindings."""
        # --- 1. Clear previous state ---
        for item in self.bound_preview_items:
            self.scene.removeItem(item)
        self.bound_preview_items.clear()

        self.preview_base_item.setVisible(False)
        self.preview_top_item.setVisible(False)

        # --- 2. Get current selections and base data ---
        base_name = self.preview_base_combo.currentText()
        top_name = self.preview_top_combo.currentText()
        base_offset = self.manifest["layers"].get(base_name, {}).get("offset", [0, 0])
        z_index = 0

        # --- 3. Render base layer ---
        if base_name and base_name != "<None>":
            base_path = self.project_path / "layers" / base_name
            if base_path.exists():
                self.preview_base_item.setPixmap(QtGui.QPixmap(str(base_path)))
                self.preview_base_item.setPos(0, 0)
                self.preview_base_item.setZValue(z_index)
                self.preview_base_item.setVisible(True)
        z_index += 1

        # --- 4. Render main top layer ---
        if top_name and top_name != "<None>":
            top_path = self.project_path / "layers" / top_name
            if top_path.exists():
                original_pixmap = QtGui.QPixmap(str(top_path))
                self.preview_top_item.setOriginalPixmap(original_pixmap)

                top_metadata = self.load_metadata_json_for_layer(top_name)
                meta_x, meta_y, meta_scale, meta_opacity = 0, 0, 1.0, 1.0
                if top_metadata and "top_layer" in top_metadata:
                    meta = top_metadata["top_layer"]
                    meta_x, meta_y = meta.get("x", 0), meta.get("y", 0)
                    meta_scale, meta_opacity = (
                        meta.get("scale", 1.0),
                        meta.get("opacity", 1.0),
                    )

                final_x = base_offset[0] + meta_x
                final_y = base_offset[1] + meta_y

                scaled_size = QtCore.QSize(
                    int(original_pixmap.width() * meta_scale),
                    int(original_pixmap.height() * meta_scale),
                )
                scaled_pixmap = original_pixmap.scaled(
                    scaled_size,
                    QtCore.Qt.KeepAspectRatio,
                    QtCore.Qt.SmoothTransformation,
                )

                self.preview_top_item.setPixmap(scaled_pixmap)
                self.preview_top_item.setPos(final_x, final_y)
                self.preview_top_item.setOpacity(meta_opacity)
                self.preview_top_item.setZValue(z_index)
                self.preview_top_item.setVisible(True)

        # --- 5. Render bound layers in specified order ---
        # The z-index continues to increment to ensure correct stacking.
        # Based on the request: render top's bindings, then base's bindings on top of those.

        # Render layers bound to the TOP layer first (at lower z-indices)
        if top_name and top_name != "<None>":
            z_index = self._render_bound_layers_for(top_name, base_offset, z_index + 1)

        # Render layers bound to the BASE layer second (at higher z-indices, so on top)
        if base_name and base_name != "<None>":
            z_index = self._render_bound_layers_for(base_name, base_offset, z_index + 1)

        # --- 6. Adjust view ---
        if fit_view:
            visible_items = [item for item in self.scene.items() if item.isVisible()]
            if visible_items:
                rect = self.scene.itemsBoundingRect()
                self.scene.setSceneRect(rect.adjusted(-20, -20, 20, 20))
                self.view.fitInView(rect, QtCore.Qt.KeepAspectRatio)
            else:
                self.scene.setSceneRect(QtCore.QRectF())

    def _render_bound_layers_for(self, source_layer_name, base_offset, start_z):
        """Helper to find and render all layers bound to a source layer."""
        current_z = start_z
        source_layer_data = self.manifest["layers"].get(source_layer_name, {})
        bound_layer_names = source_layer_data.get("bindings", [])

        for bound_name in bound_layer_names:
            bound_path = self.project_path / "layers" / bound_name
            if not bound_path.exists():
                continue

            pixmap = QtGui.QPixmap(str(bound_path))
            bound_item = QtWidgets.QGraphicsPixmapItem(pixmap)

            # Get metadata for positioning the bound item
            metadata = self.load_metadata_json_for_layer(bound_name)
            meta_x, meta_y, meta_scale, meta_opacity = 0, 0, 1.0, 1.0
            if metadata and "top_layer" in metadata:
                meta = metadata["top_layer"]
                meta_x, meta_y = meta.get("x", 0), meta.get("y", 0)
                meta_scale, meta_opacity = (
                    meta.get("scale", 1.0),
                    meta.get("opacity", 1.0),
                )

            final_x = base_offset[0] + meta_x
            final_y = base_offset[1] + meta_y

            bound_item.setPos(final_x, final_y)
            bound_item.setTransformOriginPoint(bound_item.boundingRect().center())
            bound_item.setScale(meta_scale)
            bound_item.setOpacity(meta_opacity)
            bound_item.setZValue(current_z)

            self.scene.addItem(bound_item)
            self.bound_preview_items.append(bound_item)
            current_z += 1

        return current_z

    def handle_preview_item_change(self, item):
        """
        Callback from ResizablePixmapItem. Updates metadata when an item is
        dragged or resized. Creates metadata if it doesn't exist.
        """
        top_name = self.preview_top_combo.currentText()
        if not top_name or top_name == "<None>" or not self.project_path:
            return

        layer_data = self.manifest["layers"].get(top_name, {})
        metadata_filename = layer_data.get("metadata")

        if not metadata_filename:
            metadata_filename = self._create_and_assign_metadata(top_name)
            if not metadata_filename:
                self.statusBar().showMessage(
                    f"Error: Could not create metadata for '{top_name}'.", 3000
                )
                return

            current_list_item = self.layer_list.currentItem()
            if (
                current_list_item
                and self.get_layer_name_from_item(current_list_item) == top_name
            ):
                current_list_item.setText(
                    f"{self.get_prefix_for_layer(top_name)} {top_name}"
                )
                self.on_layer_selection_changed(current_list_item, None)
            else:
                self.update_ui_state()

        metadata_json = self.load_metadata_json_for_layer(top_name)
        if not metadata_json:
            self.statusBar().showMessage(
                f"Error: Could not parse '{metadata_filename}'.", 3000
            )
            return

        base_name = self.preview_base_combo.currentText()
        base_offset = (
            self.manifest["layers"].get(base_name, {}).get("offset", [0, 0])
            if base_name and base_name != "<None>"
            else [0, 0]
        )

        # Calculate new position and scale
        pos = item.pos()
        new_meta_x = int(pos.x() - base_offset[0])
        new_meta_y = int(pos.y() - base_offset[1])

        original_pixmap = item.originalPixmap()
        current_pixmap = item.pixmap()
        new_scale = 1.0
        if (
            original_pixmap
            and not original_pixmap.isNull()
            and original_pixmap.width() > 0
        ):
            new_scale = current_pixmap.width() / original_pixmap.width()

        # Update JSON object
        top_layer_info = metadata_json.setdefault("top_layer", {})
        top_layer_info["x"] = new_meta_x
        top_layer_info["y"] = new_meta_y
        top_layer_info["scale"] = round(new_scale, 3)

        if original_pixmap and not original_pixmap.isNull():
            top_layer_info["original_width"] = original_pixmap.width()
            top_layer_info["original_height"] = original_pixmap.height()
            top_layer_info["scaled_width"] = current_pixmap.width()
            top_layer_info["scaled_height"] = current_pixmap.height()

        # Save the metadata file
        metadata_path = self.project_path / "metadata" / metadata_filename
        try:
            with open(metadata_path, "w", encoding="utf-8") as f:
                json.dump(metadata_json, f, indent=4)
            self.mark_dirty()
        except (IOError, TypeError) as e:
            QtWidgets.QMessageBox.critical(
                self,
                "Save Error",
                f"Could not write metadata to file {metadata_path}.\n\nError: {e}",
            )
            return

        # Update the UI if the modified layer is the one selected
        current_list_item = self.layer_list.currentItem()
        if (
            current_list_item
            and self.get_layer_name_from_item(current_list_item) == top_name
        ):
            widgets_to_block = [
                self.metadata_x_spinbox,
                self.metadata_y_spinbox,
                self.metadata_scale_spinbox,
                self.metadata_editor,
            ]
            for w in widgets_to_block:
                w.blockSignals(True)

            self.metadata_x_spinbox.setValue(new_meta_x)
            self.metadata_y_spinbox.setValue(new_meta_y)
            self.metadata_scale_spinbox.setValue(new_scale)
            self.metadata_editor.setPlainText(json.dumps(metadata_json, indent=4))

            for w in widgets_to_block:
                w.blockSignals(False)

    def save_metadata_from_editor(self):
        """Saves the content of the metadata editor to the corresponding file."""
        current_item = self.layer_list.currentItem()
        if not current_item:
            QtWidgets.QMessageBox.warning(self, "Warning", "No layer selected.")
            return

        layer_name = self.get_layer_name_from_item(current_item)
        metadata_filename = self.manifest["layers"].get(layer_name, {}).get("metadata")

        if not metadata_filename:
            QtWidgets.QMessageBox.warning(
                self, "Warning", "Selected layer has no associated metadata file."
            )
            return

        metadata_path = self.project_path / "metadata" / metadata_filename
        editor_content = self.metadata_editor.toPlainText()

        try:
            json.loads(editor_content)
        except json.JSONDecodeError as e:
            QtWidgets.QMessageBox.critical(
                self,
                "Invalid JSON",
                f"Could not save metadata. The content is not valid JSON.\n\nError: {e}",
            )
            return

        try:
            with open(metadata_path, "w", encoding="utf-8") as f:
                f.write(editor_content)
            self.mark_dirty()
            self.statusBar().showMessage(f"Saved changes to {metadata_filename}", 3000)

            self.on_layer_selection_changed(current_item, None)
            self.update_preview()
        except IOError as e:
            QtWidgets.QMessageBox.critical(
                self,
                "Save Error",
                f"Could not write to file {metadata_path}.\n\nError: {e}",
            )

    # --- Start of Binding Management ---

    def bind_layers(self):
        """Opens a dialog to bind one or more layers to the selected layer."""
        current_item = self.layer_list.currentItem()
        if not current_item:
            return
        source_layer_name = self.get_layer_name_from_item(current_item)
        source_layer_data = self.manifest["layers"][source_layer_name]

        # Get a list of layers that can be bound
        already_bound = set(source_layer_data.get("bindings", []))
        available_layers = []
        for name, data in self.manifest["layers"].items():
            if name == source_layer_name:
                continue
            if name in already_bound:
                continue
            # Any layer can be bound, but typically non-base layers are used.
            # We can filter here if needed, e.g., if data.get("type") != "base_layer"
            available_layers.append(name)

        if not available_layers:
            QtWidgets.QMessageBox.information(
                self, "No Layers to Bind", "There are no available layers to bind."
            )
            return

        dialog = BindLayersDialog(sorted(available_layers), self)
        if dialog.exec_() == QtWidgets.QDialog.Accepted:
            layers_to_bind = dialog.selected_layers()
            if layers_to_bind:
                bindings = source_layer_data.setdefault("bindings", [])
                for layer in layers_to_bind:
                    if layer not in bindings:
                        bindings.append(layer)

                self.mark_dirty()
                # Refresh the UI list
                self.bound_layers_list.clear()
                self.bound_layers_list.addItems(sorted(bindings))
                self.update_preview()

    def unbind_layers(self):
        """Unbinds the selected layers from the main selected layer."""
        current_item = self.layer_list.currentItem()
        if not current_item:
            return

        selected_bindings = self.bound_layers_list.selectedItems()
        if not selected_bindings:
            QtWidgets.QMessageBox.warning(
                self,
                "No Selection",
                "Select one or more layers from the 'Bound Layers' list to unbind.",
            )
            return

        source_layer_name = self.get_layer_name_from_item(current_item)
        source_layer_data = self.manifest["layers"][source_layer_name]

        if "bindings" in source_layer_data:
            for item in selected_bindings:
                layer_to_remove = item.text()
                if layer_to_remove in source_layer_data["bindings"]:
                    source_layer_data["bindings"].remove(layer_to_remove)

            # If the bindings list is now empty, remove the key
            if not source_layer_data["bindings"]:
                del source_layer_data["bindings"]

            self.mark_dirty()
            # Refresh the UI list
            self.on_layer_selection_changed(current_item, None)
            self.update_preview()

    # --- End of Binding Management ---

    # --- Start of Integrated Tools ---

    def run_migration_script(self):
        """
        Runs a migration process on the currently open project to add image
        dimension fields to older-style metadata files.
        """
        if not self.project_path:
            QtWidgets.QMessageBox.warning(
                self, "No Project", "Please open a project first."
            )
            return

        metadata_dir = self.project_path / "metadata"
        layers_dir = self.project_path / "layers"

        metadata_to_layer_map = {}
        for layer_name, layer_info in self.manifest.get("layers", {}).items():
            if "metadata" in layer_info and layer_info["metadata"]:
                metadata_filename = layer_info["metadata"]
                metadata_to_layer_map[metadata_filename] = layer_name

        processed_count = 0
        skipped_count = 0
        error_count = 0
        error_messages = []

        metadata_files = list(metadata_dir.glob("*.json"))
        if not metadata_files:
            QtWidgets.QMessageBox.information(
                self,
                "Migration Info",
                "No .json files found in the metadata directory.",
            )
            return

        progress_dialog = QtWidgets.QProgressDialog(
            "Running migration on current project...",
            "Cancel",
            0,
            len(metadata_files),
            self,
        )
        progress_dialog.setWindowModality(QtCore.Qt.WindowModal)
        progress_dialog.show()

        for i, json_path in enumerate(metadata_files):
            progress_dialog.setValue(i)
            progress_dialog.setLabelText(f"Processing '{json_path.name}'...")
            if progress_dialog.wasCanceled():
                break

            try:
                with open(json_path, "r", encoding="utf-8") as f:
                    data = json.load(f)

                if "top_layer" not in data or "original_width" in data["top_layer"]:
                    skipped_count += 1
                    continue

                metadata_filename = json_path.name
                if metadata_filename not in metadata_to_layer_map:
                    skipped_count += 1  # Not an error, just unlinked metadata
                    continue

                layer_filename = metadata_to_layer_map[metadata_filename]
                image_path = layers_dir / layer_filename

                if not image_path.exists():
                    error_count += 1
                    error_messages.append(
                        f"Layer '{layer_filename}' (for '{metadata_filename}') not found."
                    )
                    continue

                with Image.open(image_path) as img:
                    original_w, original_h = img.size

                scale = data["top_layer"].get("scale", 1.0)

                data["top_layer"]["original_width"] = original_w
                data["top_layer"]["original_height"] = original_h
                data["top_layer"]["scaled_width"] = int(original_w * scale)
                data["top_layer"]["scaled_height"] = int(original_h * scale)

                with open(json_path, "w", encoding="utf-8") as f:
                    json.dump(data, f, indent=4)

                processed_count += 1

            except json.JSONDecodeError:
                error_count += 1
                error_messages.append(f"Invalid JSON in '{json_path.name}'.")
            except Exception as e:
                error_count += 1
                error_messages.append(f"Error with '{json_path.name}': {e}")

        progress_dialog.setValue(len(metadata_files))

        current_item = self.layer_list.currentItem()
        if current_item:
            self.on_layer_selection_changed(current_item, None)

        summary_title = "Migration Complete"
        summary_message = (
            f"Files successfully migrated: {processed_count}\n"
            f"Files skipped (already new/unlinked): {skipped_count}\n"
            f"Files with errors: {error_count}"
        )

        if error_messages:
            summary_message += "\n\nError Details:\n" + "\n".join(error_messages)

        QtWidgets.QMessageBox.information(self, summary_title, summary_message)

    def cleanup_manifest(self):
        """
        Scans the manifest for layer entries whose image files no longer exist
        in the 'layers' directory and removes them. Runs on the currently open project.
        """
        if not self.project_path:
            QtWidgets.QMessageBox.warning(
                self, "No Project", "Please open a project first."
            )
            return

        reply = QtWidgets.QMessageBox.question(
            self,
            "Confirm Manifest Cleanup",
            "This will scan for and remove any layers in the manifest that no longer have a corresponding image file in the 'layers' folder.\n\nThis action cannot be undone (but changes won't be saved until you press 'Save').\n\nAre you sure you want to proceed?",
            QtWidgets.QMessageBox.Yes | QtWidgets.QMessageBox.No,
            QtWidgets.QMessageBox.No,
        )

        if reply == QtWidgets.QMessageBox.No:
            return

        layers_dir = self.project_path / "layers"
        all_layers = list(self.manifest.get("layers", {}).keys())
        missing_layers = []

        for layer_name in all_layers:
            layer_path = layers_dir / layer_name
            if not layer_path.exists():
                missing_layers.append(layer_name)

        if not missing_layers:
            QtWidgets.QMessageBox.information(
                self,
                "Cleanup Complete",
                "No orphaned layer entries were found in the manifest.",
            )
            return

        for layer_name in missing_layers:
            del self.manifest["layers"][layer_name]

        self.mark_dirty()
        self.populate_layer_list()
        self.update_ui_state()

        summary_message = (
            f"Cleanup complete. Removed {len(missing_layers)} orphaned layer entries from the manifest:\n\n- "
            + "\n- ".join(missing_layers)
        )
        QtWidgets.QMessageBox.information(self, "Cleanup Summary", summary_message)

    # --- End of Integrated Tools ---


if __name__ == "__main__":
    app = QtWidgets.QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec_())
