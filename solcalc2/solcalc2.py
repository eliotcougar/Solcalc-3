import tkinter as tk
from tkinter import ttk, messagebox

class SolutionCalculator(tk.Tk):
    def __init__(self):
        super().__init__()
        self.title("Chemical Solution Calculator")
        self.geometry("500x400")
        self.resizable(False, False)
        self.create_widgets()

    def create_widgets(self):
        self.mode_var = tk.StringVar(value="main")
        # Store all field values and units for all modes
        self.state = {
            'main': {},
            'dilution': {},
            'stock': {}
        }
        modes = [
            ("Mass/Volume Calculator", "main"),
            ("Dilution Calculator", "dilution"),
            ("Stock Calculator", "stock")
        ]
        mode_frame = ttk.LabelFrame(self, text="Mode")
        mode_frame.pack(fill="x", padx=10, pady=10)
        for text, value in modes:
            ttk.Radiobutton(mode_frame, text=text, variable=self.mode_var, value=value, command=self.update_mode).pack(side="left", padx=5, pady=5)

        self.input_frame = ttk.Frame(self)
        self.input_frame.pack(fill="both", expand=True, padx=10, pady=10)
        self.result_var = tk.StringVar()
        self.result_label = ttk.Label(self, textvariable=self.result_var, font=("Arial", 12, "bold"))
        self.result_label.pack(pady=10)
        self.help_var = tk.StringVar()
        self.help_label = ttk.Label(self, textvariable=self.help_var, font=("Arial", 9), foreground="gray")
        self.help_label.pack(pady=5)
        self.update_mode()

    def clear_frame(self):
        for widget in self.input_frame.winfo_children():
            widget.destroy()

    def save_state(self, mode):
        # Save all field values and units for the current mode
        state = {}
        if mode == 'main':
            for k in ['mm_var','vol_var','mass_var','conc_var','mm_unit','vol_unit','mass_unit','conc_unit']:
                v = getattr(self, k, None)
                if v: state[k] = v.get()
        elif mode == 'dilution':
            for k in ['stock_var','desired_var','finalvol_var','stock_unit','desired_unit','finalvol_unit']:
                v = getattr(self, k, None)
                if v: state[k] = v.get()
        elif mode == 'stock':
            for k in ['samplevol_var','stockaddvol_var','numsamples_var','samplevol_unit','stockaddvol_unit','stockconc_var','stockconc_unit','stockvol_var','stockvol_unit','mm_var','mass_var','vol_var','conc_var','mm_unit','mass_unit','vol_unit','conc_unit']:
                v = getattr(self, k, None)
                if v: state[k] = v.get()
        self.state[mode] = state

    def restore_state(self, mode):
        # Restore all field values and units for the given mode
        state = self.state.get(mode, {})
        for k, v in state.items():
            if hasattr(self, k):
                getattr(self, k).set(v)

    def update_mode(self):
        # Save current mode state
        self.save_state(self.mode_var.get())
        self.clear_frame()
        mode = self.mode_var.get()
        if mode == "main":
            self.create_main_widgets()
            self.help_var.set("Enter molecular mass, concentration, and either mass or volume to calculate the other. Units are selectable.")
        elif mode == "dilution":
            self.create_dilution_widgets()
            self.help_var.set("Calculate how to dilute a stock solution to a desired concentration and volume. Units are selectable.")
        elif mode == "stock":
            self.create_stock_widgets()
            self.help_var.set("Calculate the required stock concentration and volume for multiple samples. Optionally, calculate how to prepare the stock solution.")
        # Restore state for the new mode
        self.restore_state(mode)
        # Trigger calculation
        if mode == 'main':
            self.auto_calculate_main()
        elif mode == 'dilution':
            self.auto_calculate_dilution()
        elif mode == 'stock':
            self.auto_calculate_stock()

    def create_main_widgets(self):
        # Field labels
        ttk.Label(self.input_frame, text="Molecular Mass:").grid(row=0, column=0, sticky="e", pady=5)
        ttk.Label(self.input_frame, text="Volume:").grid(row=1, column=0, sticky="e", pady=5)
        ttk.Label(self.input_frame, text="Mass:").grid(row=2, column=0, sticky="e", pady=5)
        ttk.Label(self.input_frame, text="Concentration:").grid(row=3, column=0, sticky="e", pady=5)
        # Entry fields
        self.mm_var = tk.StringVar()
        self.vol_var = tk.StringVar()
        self.mass_var = tk.StringVar()
        self.conc_var = tk.StringVar()
        self.mm_entry = ttk.Entry(self.input_frame, textvariable=self.mm_var)
        self.vol_entry = ttk.Entry(self.input_frame, textvariable=self.vol_var)
        self.mass_entry = ttk.Entry(self.input_frame, textvariable=self.mass_var)
        self.conc_entry = ttk.Entry(self.input_frame, textvariable=self.conc_var)
        self.mm_entry.grid(row=0, column=1, pady=5)
        self.vol_entry.grid(row=1, column=1, pady=5)
        self.mass_entry.grid(row=2, column=1, pady=5)
        self.conc_entry.grid(row=3, column=1, pady=5)
        # Unit selectors
        self.mm_unit = tk.StringVar(value="g/mol")
        self.vol_unit = tk.StringVar(value="L")
        self.mass_unit = tk.StringVar(value="g")
        self.conc_unit = tk.StringVar(value="M")
        ttk.OptionMenu(self.input_frame, self.mm_unit, "g/mol", "g/mol").grid(row=0, column=2, padx=5)
        ttk.OptionMenu(self.input_frame, self.vol_unit, "L", "L", "mL", "uL").grid(row=1, column=2, padx=5)
        ttk.OptionMenu(self.input_frame, self.mass_unit, "g", "g", "mg").grid(row=2, column=2, padx=5)
        ttk.OptionMenu(self.input_frame, self.conc_unit, "M", "M", "mM", "uM").grid(row=3, column=2, padx=5)
        # Trace changes
        for var in [self.mm_var, self.vol_var, self.mass_var, self.conc_var, self.mm_unit, self.vol_unit, self.mass_unit, self.conc_unit]:
            var.trace_add('write', lambda *args: self.auto_calculate_main())
        self.result_var.set("")

    def auto_calculate_main(self):
        try:
            mm = self._to_float(self.mm_var.get())
            vol = self._to_float(self.vol_var.get())
            mass = self._to_float(self.mass_var.get())
            conc = self._to_float(self.conc_var.get())
            mm = mm if mm is not None else 0
            # Convert units to base
            vol_L = self._convert_volume_to_L(vol, self.vol_unit.get()) if vol is not None else None
            mass_g = self._convert_mass_to_g(mass, self.mass_unit.get()) if mass is not None else None
            conc_M = self._convert_conc_to_M(conc, self.conc_unit.get()) if conc is not None else None
            # Decide what to calculate
            if (mass_g is None or mass_g == 0) and vol_L:
                # Calculate mass
                if mm and conc_M:
                    result = mm * vol_L * conc_M
                    result_disp = self._convert_g_to_unit(result, self.mass_unit.get())
                    self.result_var.set(f"Required mass: {result_disp:.4f} {self.mass_unit.get()}")
                else:
                    self.result_var.set("")
            elif (vol_L is None or vol_L == 0) and mass_g:
                # Calculate volume
                if mm and conc_M:
                    result = mass_g / (mm * conc_M)
                    result_disp = self._convert_L_to_unit(result, self.vol_unit.get())
                    self.result_var.set(f"Required volume: {result_disp:.4f} {self.vol_unit.get()}")
                else:
                    self.result_var.set("")
            else:
                self.result_var.set("")
        except Exception:
            self.result_var.set("")

    def _to_float(self, value):
        try:
            return float(value)
        except Exception:
            return None

    def _convert_volume_to_L(self, value, unit):
        if value is None:
            return None
        if unit == "L":
            return value
        elif unit == "mL":
            return value / 1000
        elif unit == "uL":
            return value / 1_000_000
        return value

    def _convert_L_to_unit(self, value, unit):
        if value is None:
            return None
        if unit == "L":
            return value
        elif unit == "mL":
            return value * 1000
        elif unit == "uL":
            return value * 1_000_000
        return value

    def _convert_mass_to_g(self, value, unit):
        if value is None:
            return None
        if unit == "g":
            return value
        elif unit == "mg":
            return value / 1000
        return value

    def _convert_g_to_unit(self, value, unit):
        if value is None:
            return None
        if unit == "g":
            return value
        elif unit == "mg":
            return value * 1000
        return value

    def _convert_conc_to_M(self, value, unit):
        if value is None:
            return None
        if unit == "M":
            return value
        elif unit == "mM":
            return value / 1000
        elif unit == "uM":
            return value / 1_000_000
        return value

    def _convert_M_to_unit(self, value, unit):
        if value is None:
            return None
        if unit == "M":
            return value
        elif unit == "mM":
            return value * 1000
        elif unit == "uM":
            return value * 1_000_000
        return value

    def create_dilution_widgets(self):
        ttk.Label(self.input_frame, text="Stock Concentration:").grid(row=0, column=0, sticky="e", pady=5)
        ttk.Label(self.input_frame, text="Desired Concentration:").grid(row=1, column=0, sticky="e", pady=5)
        ttk.Label(self.input_frame, text="Final Volume:").grid(row=2, column=0, sticky="e", pady=5)
        # Entry fields
        self.stock_var = tk.StringVar()
        self.desired_var = tk.StringVar()
        self.finalvol_var = tk.StringVar()
        self.stock_entry = ttk.Entry(self.input_frame, textvariable=self.stock_var)
        self.desired_entry = ttk.Entry(self.input_frame, textvariable=self.desired_var)
        self.finalvol_entry = ttk.Entry(self.input_frame, textvariable=self.finalvol_var)
        self.stock_entry.grid(row=0, column=1, pady=5)
        self.desired_entry.grid(row=1, column=1, pady=5)
        self.finalvol_entry.grid(row=2, column=1, pady=5)
        # Unit selectors
        self.stock_unit = tk.StringVar(value="M")
        self.desired_unit = tk.StringVar(value="M")
        self.finalvol_unit = tk.StringVar(value="L")
        ttk.OptionMenu(self.input_frame, self.stock_unit, "M", "M", "mM", "uM").grid(row=0, column=2, padx=5)
        ttk.OptionMenu(self.input_frame, self.desired_unit, "M", "M", "mM", "uM").grid(row=1, column=2, padx=5)
        ttk.OptionMenu(self.input_frame, self.finalvol_unit, "L", "L", "mL", "uL").grid(row=2, column=2, padx=5)
        # Trace changes
        for var in [self.stock_var, self.desired_var, self.finalvol_var, self.stock_unit, self.desired_unit, self.finalvol_unit]:
            var.trace_add('write', lambda *args: self.auto_calculate_dilution())
        self.result_var.set("")

    def auto_calculate_dilution(self):
        try:
            stock = self._to_float(self.stock_var.get())
            desired = self._to_float(self.desired_var.get())
            finalvol = self._to_float(self.finalvol_var.get())
            stock_M = self._convert_conc_to_M(stock, self.stock_unit.get()) if stock is not None else None
            desired_M = self._convert_conc_to_M(desired, self.desired_unit.get()) if desired is not None else None
            finalvol_L = self._convert_volume_to_L(finalvol, self.finalvol_unit.get()) if finalvol is not None else None
            if not stock_M or not desired_M or not finalvol_L:
                self.result_var.set("")
                return
            if stock_M <= 0 or desired_M <= 0 or finalvol_L <= 0:
                self.result_var.set("")
                return
            if desired_M > stock_M:
                self.result_var.set("Desired concentration cannot exceed stock concentration.")
                return
            stock_vol = (desired_M * finalvol_L) / stock_M
            solvent_vol = finalvol_L - stock_vol
            stock_vol_disp = self._convert_L_to_unit(stock_vol, self.finalvol_unit.get())
            solvent_vol_disp = self._convert_L_to_unit(solvent_vol, self.finalvol_unit.get())
            self.result_var.set(f"Use {stock_vol_disp:.4f} {self.finalvol_unit.get()} stock + {solvent_vol_disp:.4f} {self.finalvol_unit.get()} solvent")
        except Exception:
            self.result_var.set("")

    def create_stock_widgets(self):
        # Main stock calculation fields
        ttk.Label(self.input_frame, text="Total sample volume:").grid(row=0, column=0, sticky="e", pady=5)
        ttk.Label(self.input_frame, text="Volume of stock to add:").grid(row=1, column=0, sticky="e", pady=5)
        ttk.Label(self.input_frame, text="Number of samples:").grid(row=2, column=0, sticky="e", pady=5)
        self.samplevol_var = tk.StringVar()
        self.stockaddvol_var = tk.StringVar()
        self.numsamples_var = tk.StringVar()
        self.samplevol_entry = ttk.Entry(self.input_frame, textvariable=self.samplevol_var)
        self.stockaddvol_entry = ttk.Entry(self.input_frame, textvariable=self.stockaddvol_var)
        self.numsamples_entry = ttk.Entry(self.input_frame, textvariable=self.numsamples_var)
        self.samplevol_entry.grid(row=0, column=1, pady=5)
        self.stockaddvol_entry.grid(row=1, column=1, pady=5)
        self.numsamples_entry.grid(row=2, column=1, pady=5)
        self.samplevol_unit = tk.StringVar(value="L")
        self.stockaddvol_unit = tk.StringVar(value="L")
        ttk.OptionMenu(self.input_frame, self.samplevol_unit, "L", "L", "mL", "uL").grid(row=0, column=2, padx=5)
        ttk.OptionMenu(self.input_frame, self.stockaddvol_unit, "L", "L", "mL", "uL").grid(row=1, column=2, padx=5)
        # Output: required stock concentration and minimum required stock volume
        ttk.Label(self.input_frame, text="Required stock concentration:").grid(row=3, column=0, sticky="e", pady=5)
        self.stockconc_var = tk.StringVar()
        self.stockconc_unit = tk.StringVar(value="M")
        self.stockconc_entry = ttk.Entry(self.input_frame, textvariable=self.stockconc_var, state='readonly')
        self.stockconc_entry.grid(row=3, column=1, pady=5)
        ttk.OptionMenu(self.input_frame, self.stockconc_unit, "M", "M", "mM", "uM").grid(row=3, column=2, padx=5)
        ttk.Label(self.input_frame, text="Required stock volume:").grid(row=4, column=0, sticky="e", pady=5)
        self.stockvol_var = tk.StringVar()
        self.stockvol_unit = tk.StringVar(value="L")
        self.stockvol_entry = ttk.Entry(self.input_frame, textvariable=self.stockvol_var)
        self.stockvol_entry.grid(row=4, column=1, pady=5)
        ttk.OptionMenu(self.input_frame, self.stockvol_unit, "L", "L", "mL", "uL").grid(row=4, column=2, padx=5)
        # Optional: stock preparation fields (reuse main mode fields)
        ttk.Label(self.input_frame, text="--- Optional: Prepare stock solution ---").grid(row=5, column=0, columnspan=3, pady=5)
        ttk.Label(self.input_frame, text="Molecular Mass:").grid(row=6, column=0, sticky="e", pady=2)
        ttk.Label(self.input_frame, text="Mass:").grid(row=7, column=0, sticky="e", pady=2)
        ttk.Label(self.input_frame, text="Volume:").grid(row=8, column=0, sticky="e", pady=2)
        ttk.Label(self.input_frame, text="Concentration:").grid(row=9, column=0, sticky="e", pady=2)
        self.mm_var = tk.StringVar()
        self.mass_var = tk.StringVar()
        self.vol_var = tk.StringVar()
        self.conc_var = tk.StringVar()
        self.mm_entry = ttk.Entry(self.input_frame, textvariable=self.mm_var)
        self.mass_entry = ttk.Entry(self.input_frame, textvariable=self.mass_var)
        self.vol_entry = ttk.Entry(self.input_frame, textvariable=self.vol_var)
        self.conc_entry = ttk.Entry(self.input_frame, textvariable=self.conc_var)
        self.mm_entry.grid(row=6, column=1, pady=2)
        self.mass_entry.grid(row=7, column=1, pady=2)
        self.vol_entry.grid(row=8, column=1, pady=2)
        self.conc_entry.grid(row=9, column=1, pady=2)
        self.mm_unit = tk.StringVar(value="g/mol")
        self.mass_unit = tk.StringVar(value="g")
        self.vol_unit = tk.StringVar(value="L")
        self.conc_unit = tk.StringVar(value="M")
        ttk.OptionMenu(self.input_frame, self.mm_unit, "g/mol", "g/mol").grid(row=6, column=2, padx=5)
        ttk.OptionMenu(self.input_frame, self.mass_unit, "g", "g", "mg").grid(row=7, column=2, padx=5)
        ttk.OptionMenu(self.input_frame, self.vol_unit, "L", "L", "mL", "uL").grid(row=8, column=2, padx=5)
        ttk.OptionMenu(self.input_frame, self.conc_unit, "M", "M", "mM", "uM").grid(row=9, column=2, padx=5)
        # Trace changes
        for var in [self.samplevol_var, self.stockaddvol_var, self.numsamples_var, self.samplevol_unit, self.stockaddvol_unit, self.stockconc_unit, self.stockvol_var, self.stockvol_unit, self.mm_var, self.mass_var, self.vol_var, self.conc_var, self.mm_unit, self.mass_unit, self.vol_unit, self.conc_unit]:
            var.trace_add('write', lambda *args: self.auto_calculate_stock())
        self.result_var.set("")

    def auto_calculate_stock(self):
        try:
            # Parse input
            samplevol = self._to_float(self.samplevol_var.get())
            stockaddvol = self._to_float(self.stockaddvol_var.get())
            numsamples = self._to_float(self.numsamples_var.get())
            samplevol_L = self._convert_volume_to_L(samplevol, self.samplevol_unit.get()) if samplevol is not None else None
            stockaddvol_L = self._convert_volume_to_L(stockaddvol, self.stockaddvol_unit.get()) if stockaddvol is not None else None
            n = int(numsamples) if numsamples is not None else None
            if not samplevol_L or not stockaddvol_L or not n or n <= 0:
                self.stockconc_var.set("")
                self.stockvol_var.set("")
                self.result_var.set("")
                return
            # Calculate required stock concentration (C_stock = C_final * V_total / V_stock)
            c_final = 1.0  # Assume 1 M final in the well (user can scale as needed)
            c_stock = c_final * samplevol_L / stockaddvol_L
            c_stock_disp = self._convert_M_to_unit(c_stock, self.stockconc_unit.get())
            self.stockconc_var.set(f"{c_stock_disp:.4g}")
            # Calculate minimum required stock volume
            min_stock_vol = stockaddvol_L * n
            # Add 10% and round up to nearest 0.1 mL or 0.01 L or 100 uL
            extra = min_stock_vol * 0.1
            min_stock_vol_total = min_stock_vol + extra
            # Round up
            if self.stockvol_unit.get() == "L":
                min_stock_vol_total = ((min_stock_vol_total*100)+0.99)//1/100
            elif self.stockvol_unit.get() == "mL":
                min_stock_vol_total = ((min_stock_vol_total*1000)+0.99)//1/1000
            elif self.stockvol_unit.get() == "uL":
                min_stock_vol_total = ((min_stock_vol_total*100000)+0.99)//1/100000
            min_stock_vol_disp = self._convert_L_to_unit(min_stock_vol_total, self.stockvol_unit.get())
            self.stockvol_var.set(f"{min_stock_vol_disp:.4g}")
            # Optional: calculate mass or volume for stock preparation if enough fields are filled
            mm = self._to_float(self.mm_var.get())
            mass = self._to_float(self.mass_var.get())
            vol = self._to_float(self.vol_var.get())
            conc = self._to_float(self.conc_var.get())
            mm = mm if mm is not None else 0
            vol_L = self._convert_volume_to_L(vol, self.vol_unit.get()) if vol is not None else None
            mass_g = self._convert_mass_to_g(mass, self.mass_unit.get()) if mass is not None else None
            conc_M = self._convert_conc_to_M(conc, self.conc_unit.get()) if conc is not None else None
            # If user entered a volume for stock prep, use it; else use calculated min_stock_vol_total
            prep_vol_L = vol_L if vol_L else min_stock_vol_total
            if (mass_g is None or mass_g == 0) and prep_vol_L and mm and conc_M:
                # Calculate mass
                result = mm * prep_vol_L * conc_M
                result_disp = self._convert_g_to_unit(result, self.mass_unit.get())
                self.result_var.set(f"To prepare stock: {result_disp:.4f} {self.mass_unit.get()} in {self._convert_L_to_unit(prep_vol_L, self.vol_unit.get()):.4f} {self.vol_unit.get()} at {self._convert_M_to_unit(conc_M, self.conc_unit.get()):.4g} {self.conc_unit.get()}")
            elif (vol_L is None or vol_L == 0) and mass_g and mm and conc_M:
                # Calculate volume
                result = mass_g / (mm * conc_M)
                result_disp = self._convert_L_to_unit(result, self.vol_unit.get())
                self.result_var.set(f"To prepare stock: {mass_g:.4f} {self.mass_unit.get()} in {result_disp:.4f} {self.vol_unit.get()} at {self._convert_M_to_unit(conc_M, self.conc_unit.get()):.4g} {self.conc_unit.get()}")
            else:
                self.result_var.set("")
        except Exception:
            self.stockconc_var.set("")
            self.stockvol_var.set("")
            self.result_var.set("")

if __name__ == "__main__":
    app = SolutionCalculator()
    app.mainloop()
