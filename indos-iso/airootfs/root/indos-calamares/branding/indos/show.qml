// OS 1 Calamares Slideshow
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Rectangle {
    id: root
    anchors.fill: parent
    color: "#1a1210"

    property int currentSlide: 0
    property int slideCount: 4

    Timer {
        id: slideTimer
        interval: 5000
        running: true
        repeat: true
        onTriggered: {
            currentSlide = (currentSlide + 1) % slideCount
        }
    }

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 30
        width: parent.width * 0.7

        // OS 1 Logo Text
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: "OS 1"
            font.pixelSize: 48
            font.bold: true
            color: "#d1684e"
        }

        // Subtitle
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: "Your computer, as a conversation."
            font.pixelSize: 20
            color: "#8b949e"
        }

        // Slide content
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 200

            // Slide 0: AI-First
            ColumnLayout {
                anchors.centerIn: parent
                spacing: 12
                visible: currentSlide === 0
                opacity: visible ? 1.0 : 0.0
                Behavior on opacity { NumberAnimation { duration: 400 } }

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "🧠 AI-First, Local-First"
                    font.pixelSize: 28
                    font.bold: true
                    color: "#c9d1d9"
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Your AI runs locally. No cloud required.\nOllama + local models ship with the OS."
                    font.pixelSize: 16
                    color: "#8b949e"
                    horizontalAlignment: Text.AlignHCenter
                    lineHeight: 1.4
                }
            }

            // Slide 1: Voice
            ColumnLayout {
                anchors.centerIn: parent
                spacing: 12
                visible: currentSlide === 1
                opacity: visible ? 1.0 : 0.0
                Behavior on opacity { NumberAnimation { duration: 400 } }

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "🎤 Talk to Your Desktop"
                    font.pixelSize: 28
                    font.bold: true
                    color: "#c9d1d9"
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Voice pipeline: Faster-Whisper STT + Piper TTS.\nFully offline, sub-second latency."
                    font.pixelSize: 16
                    color: "#8b949e"
                    horizontalAlignment: Text.AlignHCenter
                    lineHeight: 1.4
                }
            }

            // Slide 2: Niri
            ColumnLayout {
                anchors.centerIn: parent
                spacing: 12
                visible: currentSlide === 2
                opacity: visible ? 1.0 : 0.0
                Behavior on opacity { NumberAnimation { duration: 400 } }

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "🪟 Niri Scrollable Tiling"
                    font.pixelSize: 28
                    font.bold: true
                    color: "#c9d1d9"
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Modern Wayland compositor with infinite scrolling.\nCachyOS BORE scheduler for buttery performance."
                    font.pixelSize: 16
                    color: "#8b949e"
                    horizontalAlignment: Text.AlignHCenter
                    lineHeight: 1.4
                }
            }

            // Slide 3: Privacy
            ColumnLayout {
                anchors.centerIn: parent
                spacing: 12
                visible: currentSlide === 3
                opacity: visible ? 1.0 : 0.0
                Behavior on opacity { NumberAnimation { duration: 400 } }

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "🔒 Privacy by Design"
                    font.pixelSize: 28
                    font.bold: true
                    color: "#c9d1d9"
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "PII redaction before any API call.\nYour data stays on your machine."
                    font.pixelSize: 16
                    color: "#8b949e"
                    horizontalAlignment: Text.AlignHCenter
                    lineHeight: 1.4
                }
            }
        }

        // Slide indicators
        Row {
            Layout.alignment: Qt.AlignHCenter
            spacing: 8

            Repeater {
                model: slideCount
                Rectangle {
                    width: currentSlide === index ? 24 : 8
                    height: 8
                    radius: 4
                    color: currentSlide === index ? "#d1684e" : "#30363d"
                    Behavior on width { NumberAnimation { duration: 200 } }
                    Behavior on color { ColorAnimation { duration: 200 } }
                }
            }
        }

        // Progress hint
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: "Installing OS 1..."
            font.pixelSize: 14
            color: "#484f58"
            font.italic: true
        }
    }
}
