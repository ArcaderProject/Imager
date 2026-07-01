import {useEffect} from "react";
import {motion} from "framer-motion";

const PANEL = {
    sheet: {
        className: "sheet",
        initial: {opacity: 0, y: 24, scale: 0.96},
        animate: {opacity: 1, y: 0, scale: 1},
        exit: {opacity: 0, y: 24, scale: 0.96},
        transition: {type: "spring", stiffness: 320, damping: 26},
    },
    dialog: {
        className: "dialog",
        initial: {opacity: 0, scale: 0.9},
        animate: {opacity: 1, scale: 1},
        exit: {opacity: 0, scale: 0.9},
        transition: {type: "spring", stiffness: 320, damping: 24},
    },
};

const Modal = ({variant = "dialog", className = "", onClose, children}) => {
    useEffect(() => {
        const onKey = (e) => {
            if (e.key === "Escape") onClose();
        };
        window.addEventListener("keydown", onKey);
        return () => window.removeEventListener("keydown", onKey);
    }, [onClose]);

    const panel = PANEL[variant];
    return (
        <motion.div
            className="sheet-backdrop"
            onMouseDown={onClose}
            initial={{opacity: 0}}
            animate={{opacity: 1}}
            exit={{opacity: 0}}
        >
            <motion.div
                className={className ? `${panel.className} ${className}` : panel.className}
                onMouseDown={(e) => e.stopPropagation()}
                initial={panel.initial}
                animate={panel.animate}
                exit={panel.exit}
                transition={panel.transition}
            >
                {children}
            </motion.div>
        </motion.div>
    );
}

export default Modal;
