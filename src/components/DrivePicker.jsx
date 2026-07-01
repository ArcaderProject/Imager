import {motion} from "framer-motion";
import {HardDrive, RefreshCw, X, Loader2} from "lucide-react";
import {formatBytes} from "../lib/format.js";
import Modal from "./Modal.jsx";

const DrivePicker = ({drives, loading, onPick, onClose, onRefresh}) => {
    return (
        <Modal variant="sheet" onClose={onClose}>
            <div className="sheet__head">
                <span>Choose storage</span>
                <button className="sheet__close" onClick={onClose} aria-label="Close">
                    <X size={20}/>
                </button>
            </div>
            <div className="sheet__list">
                {loading && (
                    <div className="sheet__empty">
                        <Loader2 size={22} className="spin"/>
                        <div style={{marginTop: 8}}>Scanning for drives…</div>
                    </div>
                )}
                {!loading && drives.length === 0 && (
                    <div className="sheet__empty">
                        No removable drives found.
                        <br/>
                        Plug in a USB drive or SD card, then refresh.
                    </div>
                )}
                {!loading &&
                    drives.map((d) => (
                        <motion.button
                            key={d.path}
                            className="drive"
                            onClick={() => onPick(d)}
                            whileTap={{scale: 0.98}}
                        >
              <span className="drive__icon">
                <HardDrive size={22}/>
              </span>
                            <span style={{minWidth: 0}}>
                <div className="drive__name">{d.name}</div>
                <div className="drive__meta">
                  {formatBytes(d.size)} · {d.path}
                </div>
              </span>
                        </motion.button>
                    ))}
            </div>
            <div className="linkbar" style={{padding: "0 0 14px"}}>
                <button className="link" onClick={onRefresh}>
                    <RefreshCw size={13}/> Refresh list
                </button>
            </div>
        </Modal>
    );
}

export default DrivePicker;
