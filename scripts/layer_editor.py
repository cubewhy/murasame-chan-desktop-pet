# Install dependencies with
# uv add qtpy pyside6
import os
import sys
import json
from qtpy import QtWidgets, QtCore, QtGui


class DragDropLabel(QtWidgets.QLabel):
    """
    A QLabel subclass that accepts drag-and-drop for image files.
    It emits a fileDropped signal with the path of the dropped file.
    """

    # Define a signal that will be emitted when a file is dropped
    fileDropped = QtCore.Signal(str)

    def __init__(self, text, parent=None):
        """
        Initializes the DragDropLabel widget.

        Args:
            text (str): The initial placeholder text to display.
            parent (QWidget, optional): The parent widget. Defaults to None.
        """
        super().__init__(text, parent)
        # Enable dropping on the widget
        self.setAcceptDrops(True)
        # Set alignment and style for better appearance
        self.setAlignment(QtCore.Qt.AlignCenter)
        self.setWordWrap(True)
        self.setFrameShape(QtWidgets.QFrame.StyledPanel)
        self.setFrameShadow(QtWidgets.QFrame.Sunken)
        self.normal_style = "border: 2px dashed #aaa; padding: 5px; border-radius: 5px;"
        self.hover_style = "border: 2px dashed #0078d7; background-color: #eaf3fb;"
        self.setStyleSheet(self.normal_style)

    def dragEnterEvent(self, event: QtGui.QDragEnterEvent):
        """
        Handles the drag enter event.
        Accepts the event if it contains URLs corresponding to local image files.
        """
        mime_data = event.mimeData()
        # Check if the data contains URLs
        if mime_data.hasUrls():
            # Check if at least one URL is a local file with a supported image format
            for url in mime_data.urls():
                if url.isLocalFile():
                    file_info = QtCore.QFileInfo(url.toLocalFile())
                    ext = file_info.suffix().lower()
                    if ext in ["png", "jpg", "jpeg", "bmp"]:
                        event.acceptProposedAction()
                        self.setStyleSheet(self.hover_style)  # Highlight on hover
                        return
        event.ignore()

    def dragLeaveEvent(self, event: QtGui.QDragLeaveEvent):
        """
        Handles the drag leave event.
        Resets the stylesheet to its default state.
        """
        self.setStyleSheet(self.normal_style)

    def dropEvent(self, event: QtGui.QDropEvent):
        """
        Handles the drop event.
        Emits the fileDropped signal with the path of the first valid image file.
        """
        self.setStyleSheet(self.normal_style)  # Reset style
        mime_data = event.mimeData()
        if mime_data.hasUrls():
            for url in mime_data.urls():
                if url.isLocalFile():
                    path = url.toLocalFile()
                    file_info = QtCore.QFileInfo(path)
                    ext = file_info.suffix().lower()
                    if ext in ["png", "jpg", "jpeg", "bmp"]:
                        # Emit the signal with the file path
                        self.fileDropped.emit(path)
                        # We only handle the first valid file
                        return


class ResizablePixmapItem(QtWidgets.QGraphicsPixmapItem):
    """
    A resizable and movable QGraphicsPixmapItem.
    It draws handles and handles mouse events for scaling.
    """

    handleSize = 10.0
    handleSpace = -5.0

    def __init__(self, pixmap, parent=None):
        super().__init__(pixmap, parent)
        self._pixmap = pixmap  # Store the original pixmap
        self.handles = {}
        self.handleSelected = None
        self.mousePressPos = None
        self.mousePressRect = None
        self.setAcceptHoverEvents(True)
        self.setFlag(QtWidgets.QGraphicsItem.ItemIsMovable, True)
        self.setFlag(QtWidgets.QGraphicsItem.ItemIsSelectable, True)
        self.setFlag(QtWidgets.QGraphicsItem.ItemSendsGeometryChanges, True)
        self.setFlag(QtWidgets.QGraphicsItem.ItemIsFocusable, True)
        self.updateHandlesPos()

    def originalPixmap(self):
        """Returns the original, unscaled pixmap."""
        return self._pixmap

    def handleAt(self, point):
        """Returns the scaling handle at the given point."""
        for k, v in self.handles.items():
            if v.contains(point):
                return k
        return None

    def hoverMoveEvent(self, moveEvent):
        """Executed when the mouse hovers over the item."""
        if self.isSelected():
            handle = self.handleAt(moveEvent.pos())
            if handle:
                cursor = self.get_cursor_for_handle(handle)
                self.setCursor(cursor)
            else:
                self.setCursor(QtCore.Qt.ArrowCursor)
        super().hoverMoveEvent(moveEvent)

    def get_cursor_for_handle(self, handle):
        """Returns the appropriate cursor shape based on the handle position."""
        if handle in (QtCore.Qt.TopLeftCorner, QtCore.Qt.BottomRightCorner):
            return QtCore.Qt.SizeFDiagCursor
        if handle in (QtCore.Qt.TopRightCorner, QtCore.Qt.BottomLeftCorner):
            return QtCore.Qt.SizeBDiagCursor
        if handle in (QtCore.Qt.TopEdge, QtCore.Qt.BottomEdge):
            return QtCore.Qt.SizeVerCursor
        return QtCore.Qt.SizeHorCursor

    def mousePressEvent(self, mouseEvent):
        """Executed when the mouse is pressed."""
        self.handleSelected = self.handleAt(mouseEvent.pos())
        if self.handleSelected:
            self.mousePressPos = mouseEvent.pos()
            self.mousePressRect = self.boundingRect()
        super().mousePressEvent(mouseEvent)

    def mouseMoveEvent(self, mouseEvent):
        """Executed when the mouse is moved."""
        if self.handleSelected is not None:
            self.interactiveResize(mouseEvent.pos())
        else:
            super().mouseMoveEvent(mouseEvent)

    def mouseReleaseEvent(self, mouseEvent):
        """Executed when the mouse is released."""
        super().mouseReleaseEvent(mouseEvent)
        self.handleSelected = None
        self.mousePressPos = None
        self.mousePressRect = None
        self.update()

    def boundingRect(self):
        """
        Returns the bounding rectangle of the item, including space for handles.
        """
        o = self.handleSize + self.handleSpace
        return self.pixmap().rect().adjusted(-o, -o, o, o)

    def updateHandlesPos(self):
        """Updates the position of the scaling handles."""
        s = self.handleSize
        r = self.boundingRect()
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
        self.handles[QtCore.Qt.TopEdge] = QtCore.QRectF(
            r.center().x() - s / 2, r.top(), s, s
        )
        self.handles[QtCore.Qt.BottomEdge] = QtCore.QRectF(
            r.center().x() - s / 2, r.bottom() - s, s, s
        )
        self.handles[QtCore.Qt.LeftEdge] = QtCore.QRectF(
            r.left(), r.center().y() - s / 2, s, s
        )
        self.handles[QtCore.Qt.RightEdge] = QtCore.QRectF(
            r.right() - s, r.center().y() - s / 2, s, s
        )

    def interactiveResize(self, mousePos):
        """Resizes the Pixmap based on the mouse position."""
        self.prepareGeometryChange()

        # Get the current scene rectangle
        current_rect = self.sceneBoundingRect()
        new_rect = QtCore.QRectF(current_rect)

        # Map mouse position from item coordinates to scene coordinates
        scene_mouse_pos = self.mapToScene(mousePos)

        # Calculate the new rectangle based on the selected handle
        if self.handleSelected == QtCore.Qt.TopLeftCorner:
            new_rect.setTopLeft(scene_mouse_pos)
        elif self.handleSelected == QtCore.Qt.TopRightCorner:
            new_rect.setTopRight(scene_mouse_pos)
        elif self.handleSelected == QtCore.Qt.BottomLeftCorner:
            new_rect.setBottomLeft(scene_mouse_pos)
        elif self.handleSelected == QtCore.Qt.BottomRightCorner:
            new_rect.setBottomRight(scene_mouse_pos)
        elif self.handleSelected == QtCore.Qt.TopEdge:
            new_rect.setTop(scene_mouse_pos.y())
        elif self.handleSelected == QtCore.Qt.BottomEdge:
            new_rect.setBottom(scene_mouse_pos.y())
        elif self.handleSelected == QtCore.Qt.LeftEdge:
            new_rect.setLeft(scene_mouse_pos.x())
        elif self.handleSelected == QtCore.Qt.RightEdge:
            new_rect.setRight(scene_mouse_pos.x())

        # Set the new position and scale the pixmap
        new_rect = new_rect.normalized()
        self.setPos(new_rect.topLeft())

        # To avoid distortion, we scale the original pixmap
        scaled_pixmap = self._pixmap.scaled(
            new_rect.width(),
            new_rect.height(),
            QtCore.Qt.KeepAspectRatio,  # Changed to keep aspect ratio
            QtCore.Qt.SmoothTransformation,
        )
        self.setPixmap(scaled_pixmap)

        self.updateHandlesPos()

    def paint(self, painter, option, widget=None):
        """Paints the item."""
        # FIX: Draw the pixmap at the item's origin (0,0).
        # The item's position is handled by self.pos().
        # Drawing at boundingRect().topLeft() was causing an offset.
        painter.drawPixmap(QtCore.QPointF(0, 0), self.pixmap())

        # If selected, draw the selection rectangle and handles
        if self.isSelected():
            painter.setRenderHint(QtGui.QPainter.Antialiasing)
            painter.setBrush(QtGui.QBrush(QtGui.QColor(0, 0, 255, 100)))
            painter.setPen(
                QtGui.QPen(QtGui.QColor(0, 0, 255, 200), 2.0, QtCore.Qt.DashLine)
            )
            # FIX: Draw the rectangle around the actual pixmap, not the larger bounding rect.
            painter.drawRect(self.pixmap().rect())

            painter.setPen(QtGui.QPen(QtGui.QColor(0, 0, 255), 1.0))
            painter.setBrush(QtGui.QBrush(QtCore.Qt.white))
            for handle, rect in self.handles.items():
                painter.drawRect(rect)


class MainWindow(QtWidgets.QMainWindow):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Layer Editing Tool")
        self.setGeometry(100, 100, 1200, 800)

        self.base_layer = None
        self.top_layer = None

        self.base_layer_name: str = "<not loaded>"
        self.top_layer_name: str = "<not loaded>"

        # Main view and scene
        self.scene = QtWidgets.QGraphicsScene()
        self.view = QtWidgets.QGraphicsView(self.scene)
        self.setCentralWidget(self.view)

        self.create_actions()
        self.create_menus()
        self.create_toolbar()
        self.create_dock_widget()

        self.view.setFocusPolicy(QtCore.Qt.StrongFocus)

    def create_actions(self):
        """Create the main actions for the application."""
        self.import_base_action = QtWidgets.QAction(
            "Import Base Layer...", self, triggered=self.import_base_layer
        )
        self.import_top_action = QtWidgets.QAction(
            "Import Top Layer...", self, triggered=self.import_top_layer
        )
        self.import_json_action = QtWidgets.QAction(
            "Import from JSON...", self, triggered=self.import_from_json
        )
        self.export_json_action = QtWidgets.QAction(
            "Export to JSON...", self, triggered=self.export_to_json
        )
        self.quit_action = QtWidgets.QAction("Quit", self, triggered=self.close)

    def create_menus(self):
        """Create the main menu bar."""
        menu_bar = self.menuBar()
        file_menu = menu_bar.addMenu("File")
        file_menu.addAction(self.import_base_action)
        file_menu.addAction(self.import_top_action)
        file_menu.addSeparator()
        file_menu.addAction(self.import_json_action)
        file_menu.addAction(self.export_json_action)
        file_menu.addSeparator()
        file_menu.addAction(self.quit_action)

    def create_toolbar(self):
        """Create the main toolbar."""
        toolbar = self.addToolBar("Tools")
        toolbar.addAction(self.import_base_action)
        toolbar.addAction(self.import_top_action)
        toolbar.addAction(self.import_json_action)
        toolbar.addAction(self.export_json_action)

    def create_dock_widget(self):
        """Create the dock widget for layer properties and controls."""
        dock = QtWidgets.QDockWidget("Properties", self)
        self.addDockWidget(QtCore.Qt.RightDockWidgetArea, dock)

        scroll_area = QtWidgets.QScrollArea()
        dock.setWidget(scroll_area)

        widget = QtWidgets.QWidget()
        scroll_area.setWidget(widget)
        scroll_area.setWidgetResizable(True)

        layout = QtWidgets.QFormLayout(widget)

        self.x_spinbox = QtWidgets.QSpinBox(self)
        self.y_spinbox = QtWidgets.QSpinBox(self)
        self.scale_spinbox = QtWidgets.QDoubleSpinBox(self)
        self.opacity_slider = QtWidgets.QSlider(QtCore.Qt.Horizontal)
        self.opacity_spinbox = QtWidgets.QDoubleSpinBox(self)

        self.scale_spinbox.setRange(0.01, 100.0)
        self.scale_spinbox.setSingleStep(0.01)
        self.scale_spinbox.setValue(1)

        for spinbox in [
            self.x_spinbox,
            self.y_spinbox,
            self.scale_spinbox,
        ]:
            spinbox.setRange(-10000, 10000)
            spinbox.setEnabled(False)
        self.opacity_spinbox.setEnabled(False)

        self.opacity_slider.setRange(0, 100)
        self.opacity_slider.setValue(100)
        self.opacity_spinbox.setRange(0, 1)
        self.opacity_spinbox.setValue(1)
        self.opacity_spinbox.setSingleStep(0.01)
        self.opacity_slider.setEnabled(False)

        layout.addRow("X:", self.x_spinbox)
        layout.addRow("Y:", self.y_spinbox)
        layout.addRow("Scale:", self.scale_spinbox)
        layout.addRow("Opacity:", self.opacity_slider)
        layout.addRow("", self.opacity_spinbox)
        layout.addRow("", QtWidgets.QSplitter(self))

        # Create drag-and-drop boxes for layers
        self.base_layer_drop_box = DragDropLabel("Drag Base Layer Here", self)
        self.top_layer_drop_box = DragDropLabel("Drag Top Layer Here", self)
        layout.addRow("Base Layer:", self.base_layer_drop_box)
        layout.addRow("Top Layer:", self.top_layer_drop_box)

        # Connect signals to slots
        self.base_layer_drop_box.fileDropped.connect(self.load_base_layer)
        self.top_layer_drop_box.fileDropped.connect(self.load_top_layer)
        self.x_spinbox.valueChanged.connect(self.update_top_layer_from_controls)
        self.y_spinbox.valueChanged.connect(self.update_top_layer_from_controls)
        self.scale_spinbox.valueChanged.connect(self.update_top_layer_from_controls)
        self.opacity_slider.valueChanged.connect(
            lambda val: self.opacity_spinbox.setValue(val / 100.0)
        )
        self.opacity_spinbox.valueChanged.connect(self.update_top_layer_opacity)

    def import_base_layer(self):
        """
        Opens a file dialog to select and import the base layer image.
        """
        path, _ = QtWidgets.QFileDialog.getOpenFileName(
            self, "Select Base Image", "", "Image Files (*.png *.jpg *.jpeg *.bmp)"
        )
        if path:
            self.load_base_layer(path)

    def load_base_layer(self, path: str):
        """
        Loads the base layer image from the given file path. This method
        is used by both the file dialog and the drag-and-drop box.

        Args:
            path (str): The file path of the image to load.
        """
        self.base_layer_name = os.path.basename(path)
        self.base_layer_drop_box.setText(self.base_layer_name)
        if self.base_layer:
            self.scene.removeItem(self.base_layer)
        pixmap = QtGui.QPixmap(path)
        self.base_layer = self.scene.addPixmap(pixmap)
        self.base_layer.setZValue(0)
        # Set base layer at origin
        self.base_layer.setPos(0, 0)
        self.scene.setSceneRect(self.base_layer.boundingRect())

    def import_top_layer(self):
        """
        Opens a file dialog to select and import the top layer image.
        """
        path, _ = QtWidgets.QFileDialog.getOpenFileName(
            self, "Select Top Image", "", "Image Files (*.png *.jpg *.jpeg *.bmp)"
        )
        if path:
            self.load_top_layer(path)

    def load_top_layer(self, path: str):
        """
        Loads the top layer image from the given file path. This method
        is used by both the file dialog and the drag-and-drop box.

        Args:
            path (str): The file path of the image to load.
        """
        self.top_layer_name = os.path.basename(path)
        self.top_layer_drop_box.setText(self.top_layer_name)
        if self.top_layer:
            self.scene.removeItem(self.top_layer)

        pixmap = QtGui.QPixmap(path)
        self.top_layer = ResizablePixmapItem(pixmap)

        self.scene.addItem(self.top_layer)
        self.top_layer.setZValue(1)
        self.top_layer.setPos(50, 50)

        for spinbox in [
            self.x_spinbox,
            self.y_spinbox,
            self.scale_spinbox,
            self.opacity_spinbox,
        ]:
            spinbox.setEnabled(True)
        self.opacity_slider.setEnabled(True)

        self.top_layer.setTransformOriginPoint(self.top_layer.boundingRect().center())
        self.update_controls_from_top_layer()

        # Using a timer to continuously update controls from item's state.
        # Create it only if it doesn't exist.
        if not hasattr(self, "timer"):
            self.timer = QtCore.QTimer(self)
            self.timer.setInterval(100)
            self.timer.timeout.connect(self.update_controls_from_top_layer)
            self.timer.start()

    def update_top_layer_from_controls(self):
        """
        Updates the top layer's position and scale based on the values
        in the control spinboxes.
        """
        if self.top_layer and not self.top_layer.isSelected():
            self.top_layer.setPos(self.x_spinbox.value(), self.y_spinbox.value())

            current_pixmap = self.top_layer.originalPixmap()
            scale = self.scale_spinbox.value()
            new_width = current_pixmap.width() * scale
            new_height = current_pixmap.height() * scale

            if new_width > 0 and new_height > 0:
                self.top_layer.prepareGeometryChange()
                scaled_pixmap = current_pixmap.scaled(
                    new_width,
                    new_height,
                    QtCore.Qt.KeepAspectRatio,
                    QtCore.Qt.SmoothTransformation,
                )
                self.top_layer.setPixmap(scaled_pixmap)
                self.top_layer.updateHandlesPos()

    def update_top_layer_opacity(self):
        """Updates the top layer's opacity based on the opacity controls."""
        if self.top_layer:
            opacity = self.opacity_spinbox.value()
            self.top_layer.setOpacity(opacity)
            if not self.opacity_slider.isSliderDown():
                self.opacity_slider.setValue(int(opacity * 100))

    def update_controls_from_top_layer(self):
        """
        Updates the control spinboxes based on the current state of the
        top layer item when it is moved or resized.
        """
        if self.top_layer and self.top_layer.isSelected():
            pos = self.top_layer.pos()

            # Block signals to prevent infinite loops while updating controls
            self.x_spinbox.blockSignals(True)
            self.y_spinbox.blockSignals(True)
            self.scale_spinbox.blockSignals(True)

            self.x_spinbox.setValue(int(pos.x()))
            self.y_spinbox.setValue(int(pos.y()))

            # FIX: Calculate and update scale based on current vs original pixmap width
            original_width = self.top_layer.originalPixmap().width()
            current_width = self.top_layer.pixmap().width()
            if original_width > 0:
                scale = current_width / original_width
                self.scale_spinbox.setValue(scale)

            # Unblock signals
            self.x_spinbox.blockSignals(False)
            self.y_spinbox.blockSignals(False)
            self.scale_spinbox.blockSignals(False)

    def export_to_json(self):
        """Exports the layer properties to a JSON file."""
        if not self.base_layer or not self.top_layer:
            QtWidgets.QMessageBox.warning(
                self, "Error", "Please import both base and top layers first."
            )
            return

        # FIX: Get coordinates and dimensions correctly
        # The base layer is at (0,0). top_layer.pos() is already relative to it.
        base_rect = self.base_layer.boundingRect()
        top_pos = self.top_layer.pos()
        top_pixmap = self.top_layer.pixmap()

        data = {
            "base_layer": {
                "width": int(base_rect.width()),
                "height": int(base_rect.height()),
            },
            "top_layer": {
                "x": int(top_pos.x()),
                "y": int(top_pos.y()),
                "width": int(top_pixmap.width()),
                "height": int(top_pixmap.height()),
                "scale": self.scale_spinbox.value(),
                "opacity": self.top_layer.opacity(),
            },
        }

        path, _ = QtWidgets.QFileDialog.getSaveFileName(
            self, "Export to JSON", "", "JSON Files (*.json)"
        )
        if path:
            try:
                with open(path, "w", encoding="utf-8") as f:
                    json.dump(data, f, ensure_ascii=False, indent=4)
                QtWidgets.QMessageBox.information(
                    self, "Success", f"Data successfully exported to {path}"
                )
            except Exception as e:
                QtWidgets.QMessageBox.critical(self, "Export Failed", str(e))

    def import_from_json(self):
        """Imports and applies layer properties from a JSON file."""
        if not self.top_layer:
            QtWidgets.QMessageBox.warning(
                self, "Error", "Please import the top layer image first."
            )
            return

        path, _ = QtWidgets.QFileDialog.getOpenFileName(
            self, "Import from JSON", "", "JSON Files (*.json)"
        )
        if path:
            try:
                with open(path, "r", encoding="utf-8") as f:
                    data = json.load(f)

                top_layer_data = data.get("top_layer")
                if not top_layer_data:
                    raise ValueError("JSON does not contain 'top_layer' data.")

                # Set properties from JSON
                self.top_layer.setPos(top_layer_data["x"], top_layer_data["y"])

                new_width = top_layer_data["width"]
                new_height = top_layer_data["height"]

                if new_width > 0 and new_height > 0:
                    scaled_pixmap = self.top_layer.originalPixmap().scaled(
                        new_width,
                        new_height,
                        QtCore.Qt.KeepAspectRatio,
                        QtCore.Qt.SmoothTransformation,
                    )
                    self.top_layer.setPixmap(scaled_pixmap)

                self.top_layer.setOpacity(top_layer_data.get("opacity", 1.0))

                # Update controls
                self.x_spinbox.setValue(top_layer_data["x"])
                self.y_spinbox.setValue(top_layer_data["y"])
                self.scale_spinbox.setValue(top_layer_data["scale"])
                self.opacity_spinbox.setValue(top_layer_data.get("opacity", 1.0))

                QtWidgets.QMessageBox.information(
                    self, "Success", "Layer data imported successfully."
                )

                # trigger updates
                self.update_top_layer_from_controls()

            except Exception as e:
                QtWidgets.QMessageBox.critical(self, "Import Failed", str(e))

    def keyPressEvent(self, event):
        """Handles key press events for moving the selected layer."""
        if self.top_layer and self.top_layer.isSelected():
            step = 1
            if event.modifiers() & QtCore.Qt.ShiftModifier:
                step = 10

            if event.key() == QtCore.Qt.Key_Up:
                self.top_layer.moveBy(0, -step)
            elif event.key() == QtCore.Qt.Key_Down:
                self.top_layer.moveBy(0, step)
            elif event.key() == QtCore.Qt.Key_Left:
                self.top_layer.moveBy(-step, 0)
            elif event.key() == QtCore.Qt.Key_Right:
                self.top_layer.moveBy(step, 0)
            # No need to call update_controls here, the timer will handle it.
        else:
            super().keyPressEvent(event)


if __name__ == "__main__":
    app = QtWidgets.QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec_())
