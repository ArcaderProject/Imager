import {motion} from "framer-motion";
import {Gamepad2, FlaskConical, Disc, X, Check} from "lucide-react";
import Modal from "./Modal.jsx";

const SourcePicker = ({os, osError, active, onPickArcader, onPickLocal, onClose, onRetry}) => {
    const arcaderMeta = os
        ? `${os.version} · Stable`
        : osError
            ? "Update check failed · tap to retry"
            : "Checking latest version…";
    return (
        <Modal variant="sheet" onClose={onClose}>
            <div className="sheet__head">
                <span>Choose image</span>
                <button className="sheet__close" onClick={onClose} aria-label="Close">
                    <X size={20}/>
                </button>
            </div>
            <div className="sheet__list">
                <motion.button
                    className="drive"
                    onClick={os ? onPickArcader : onRetry}
                    whileTap={{scale: 0.98}}
                >
          <span className="drive__icon">
            <Gamepad2 size={22}/>
          </span>
                    <span style={{minWidth: 0, flex: 1}}>
            <div className="drive__name">{os ? os.name : "Arcader OS"}</div>
            <div className="drive__meta">{arcaderMeta}</div>
          </span>
                    {active === "arcader" && <Check size={18}/>}
                </motion.button>

                <div className="drive is-disabled" aria-disabled="true">
          <span className="drive__icon">
            <FlaskConical size={22}/>
          </span>
                    <span style={{minWidth: 0, flex: 1}}>
            <div className="drive__name">Arcader OS Beta</div>
            <div className="drive__meta">Bleeding-edge builds</div>
          </span>
                    <span className="soon-tag">Coming soon</span>
                </div>

                <motion.button
                    className="drive"
                    onClick={onPickLocal}
                    whileTap={{scale: 0.98}}
                >
          <span className="drive__icon">
            <Disc size={22}/>
          </span>
                    <span style={{minWidth: 0, flex: 1}}>
            <div className="drive__name">Local image…</div>
            <div className="drive__meta">
              Pick an .iso or .img from this computer
            </div>
          </span>
                    {active === "local" && <Check size={18}/>}
                </motion.button>
            </div>
        </Modal>
    );
}

export default SourcePicker;
