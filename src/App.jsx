import {useEffect, useState, useCallback} from "react";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import {AnimatePresence, motion} from "framer-motion";
import {Gamepad2, Disc, HardDrive, ChevronRight} from "lucide-react";

import logo from "./assets/img/logo.png";
import {formatBytes} from "./lib/format.js";
import TitleBar from "./components/TitleBar.jsx";
import DrivePicker from "./components/DrivePicker.jsx";
import SourcePicker from "./components/SourcePicker.jsx";
import ConfirmDialog from "./components/ConfirmDialog.jsx";
import MessageDialog from "./components/MessageDialog.jsx";
import Progress from "./components/Progress.jsx";

const IDLE = "idle";
const CONFIRM = "confirm";
const FLASHING = "flashing";

const App = () => {
    const [os, setOs] = useState(null);
    const [osError, setOsError] = useState(false);
    const [drives, setDrives] = useState([]);
    const [drivesLoading, setDrivesLoading] = useState(false);
    const [selected, setSelected] = useState(null);
    const [showPicker, setShowPicker] = useState(false);
    const [source, setSource] = useState({kind: "arcader"});
    const [showSource, setShowSource] = useState(false);
    const [stage, setStage] = useState(IDLE);
    const [progress, setProgress] = useState({
        phase: "prepare",
        done: 0,
        total: 0,
        rate: 0,
    });
    const [message, setMessage] = useState(null);

    const loadOs = useCallback(async () => {
        setOsError(false);
        try {
            setOs(await invoke("get_os_info"));
        } catch {
            setOsError(true);
        }
    }, []);

    useEffect(() => {
        loadOs();
    }, [loadOs]);

    useEffect(() => {
        const un = listen("flash://progress", (e) => setProgress(e.payload));
        return () => {
            un.then((f) => f());
        };
    }, []);

    const refreshDrives = useCallback(async () => {
        setDrivesLoading(true);
        try {
            const list = await invoke("list_drives");
            setDrives(list);
            setSelected((cur) =>
                cur && list.some((d) => d.path === cur.path) ? cur : null
            );
        } catch (err) {
            setMessage({
                kind: "error",
                title: "Cannot list drives",
                text: String(err),
            });
        } finally {
            setDrivesLoading(false);
        }
    }, []);

    const openPicker = useCallback(() => {
        setShowPicker(true);
        refreshDrives();
    }, [refreshDrives]);

    const resolveSource = useCallback(() => {
        if (source.kind === "local") {
            return {path: source.path, label: source.name};
        }
        return os ? {path: os.url, label: `${os.name} ${os.version}`} : null;
    }, [source, os]);

    const sourceReady = source.kind === "local" || !!os;

    const chooseArcader = useCallback(() => {
        setSource({kind: "arcader"});
        setShowSource(false);
    }, []);

    const chooseLocal = useCallback(async () => {
        setShowSource(false);
        try {
            const img = await invoke("pick_image_file");
            if (img) setSource({kind: "local", ...img});
        } catch (err) {
            setMessage({kind: "error", title: "Cannot open image", text: String(err)});
        }
    }, []);

    const startFlash = useCallback(async () => {
        const src = resolveSource();
        if (!src || !selected) return;
        setProgress({phase: "auth", done: 0, total: 0, rate: 0});
        setStage(FLASHING);
        try {
            await invoke("start_flash", {
                source: src.path,
                device: selected.path,
                verify: true,
            });
            setStage(IDLE);
            setMessage({
                kind: "good",
                title: "Write complete",
                text: `${src.label} was written and verified. You can remove the drive and boot your Arcader machine from it.`,
            });
        } catch (err) {
            setStage(IDLE);
            const text = String(err);
            setMessage({
                kind: "error",
                title: text === "Cancelled" ? "Cancelled" : "Write failed",
                text:
                    text === "Cancelled"
                        ? "The write was cancelled. The drive is not bootable yet."
                        : text,
            });
        }
    }, [resolveSource, selected]);

    const cancelFlash = useCallback(() => {
        invoke("cancel_flash").catch(() => {
        });
    }, []);

    if (stage === FLASHING) {
        return (
            <div className="app">
                <TitleBar/>
                <div className="app__scroll" style={{justifyContent: "center"}}>
                    <img className="header__logo" src={logo} alt="Arcader" style={{marginBottom: 34}}/>
                    <Progress progress={progress} onCancel={cancelFlash}/>
                </div>
            </div>
        );
    }

    return (
        <div className="app">
            <TitleBar/>
            <div className="app__scroll">
                <div className="header">
                    <img className="header__logo" src={logo} alt="Arcader"/>
                    <div className="header__subtitle">OS Imager</div>
                </div>

                <motion.div
                    className="field"
                    initial={{opacity: 0, y: 6}}
                    animate={{opacity: 1, y: 0}}
                    transition={{delay: 0.04, duration: 0.28}}
                >
                    <div className="field__label">
                        Image
                    </div>
                    <button className="select" onClick={() => setShowSource(true)}>
            <span className="select__icon">
              {source.kind === "local" ? <Disc size={20}/> : <Gamepad2 size={20}/>}
            </span>
                        <span className="select__body">
              <span className="select__value">
                {source.kind === "local"
                    ? source.name
                    : os
                        ? os.name
                        : osError
                            ? "Arcader OS"
                            : "Loading…"}
              </span>
              <span className="select__hint">
                {source.kind === "local"
                    ? `${formatBytes(source.size)} · local image`
                    : os
                        ? `${os.version} · Stable`
                        : osError
                            ? "Update check failed"
                            : "Checking latest version…"}
              </span>
            </span>
                        <span className="select__chev">
              <ChevronRight size={18}/>
            </span>
                    </button>
                </motion.div>

                <motion.div
                    className="field"
                    initial={{opacity: 0, y: 6}}
                    animate={{opacity: 1, y: 0}}
                    transition={{delay: 0.1, duration: 0.28}}
                >
                    <div className="field__label">
                        Storage
                    </div>
                    <button
                        className={`select${selected ? "" : " is-empty"}`}
                        onClick={openPicker}
                    >
            <span className="select__icon">
              <HardDrive size={20}/>
            </span>
                        <span className="select__body">
              <span className="select__value">
                {selected ? selected.name : "Choose storage…"}
              </span>
              <span className="select__hint">
                {selected
                    ? `${formatBytes(selected.size)} · ${selected.path}`
                    : "USB drive or SD card"}
              </span>
            </span>
                        <span className="select__chev">
              <ChevronRight size={18}/>
            </span>
                    </button>
                </motion.div>

                <motion.div
                    className="field"
                    initial={{opacity: 0, y: 6}}
                    animate={{opacity: 1, y: 0}}
                    transition={{delay: 0.16, duration: 0.28}}
                >
                    <div className="field__label">
                        Write
                    </div>
                    <button
                        className="pbtn pbtn--write"
                        disabled={!sourceReady || !selected}
                        onClick={() => setStage(CONFIRM)}
                    >
                        Write
                    </button>
                </motion.div>
            </div>

            <AnimatePresence>
                {showSource && (
                    <SourcePicker
                        key="source"
                        os={os}
                        osError={osError}
                        active={source.kind}
                        onClose={() => setShowSource(false)}
                        onPickArcader={chooseArcader}
                        onPickLocal={chooseLocal}
                        onRetry={loadOs}
                    />
                )}

                {showPicker && (
                    <DrivePicker
                        key="picker"
                        drives={drives}
                        loading={drivesLoading}
                        onRefresh={refreshDrives}
                        onClose={() => setShowPicker(false)}
                        onPick={(d) => {
                            setSelected(d);
                            setShowPicker(false);
                        }}
                    />
                )}

                {stage === CONFIRM && selected && (
                    <ConfirmDialog
                        key="confirm"
                        image={
                            source.kind === "local"
                                ? {
                                    name: source.name,
                                    detail: `${formatBytes(source.size)} · local image`,
                                }
                                : {name: os?.name, detail: os?.version}
                        }
                        drive={selected}
                        onCancel={() => setStage(IDLE)}
                        onConfirm={() => {
                            setStage(IDLE);
                            startFlash();
                        }}
                    />
                )}

                {message && (
                    <MessageDialog
                        key="message"
                        kind={message.kind}
                        title={message.title}
                        text={message.text}
                        onClose={() => setMessage(null)}
                    />
                )}
            </AnimatePresence>
        </div>
    );
}

export default App;
