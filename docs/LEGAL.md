# Legal Notice & Compliance Policy — Alolite

## 1. Trademark Notice & Nominative Fair Use

* **"Zalo"** is a registered trademark of **VNG Corporation** in the Socialist Republic of Vietnam and internationally.
* **Alolite** is an entirely independent, third-party open-source project. This software is **NOT** affiliated with, endorsed by, sponsored by, or officially associated with VNG Corporation or any of its subsidiaries.
* Any references to "Zalo" within this repository, source code identifiers, commit history, or technical documentation are made solely for **Nominative Fair Use** purposes: to accurately identify and describe the network protocol and technical compatibility target. The project name, public releases, and logos do not use VNG Corporation's trademarks as brand identifiers.

---

## 2. Copyright & Clean-Room Interoperability

Alolite is implemented as an independent, clean-room software client designed for technical research and platform interoperability on Linux operating systems.

### 2.1 Statutory Basis for Interoperability
1. **Socialist Republic of Vietnam — Law on Intellectual Property (Articles 20 & 50a):**
   Explicitly permits decompilation, reverse engineering, and technical analysis of computer programs strictly to achieve interoperability between independently created software programs, provided it does not prejudice the normal exploitation of the work.
2. **United States — 17 U.S.C. § 1201(f) (Reverse Engineering Exemption):**
   Permits reverse engineering of technological protection measures solely for the purpose of enabling interoperability of an independently created computer program with other programs.
3. **European Union — Directive 2009/24/EC (Article 6 - Decompilation):**
   Authorizes decompilation without the owner's authorization where reproduction of the code and translation of its form are indispensable to obtain the information necessary to achieve the interoperability of an independently created computer program with other programs.

### 2.2 Clean-Room Engineering Invariants
* **Zero Source Code Infringement:** No decompiled C/C++ source code, disassembly dumps, proprietary headers, or extracted JavaScript bundles from official packages are committed to this repository.
* **Independent Rust Implementation:** All cryptographic routines (`ParamCipher v2`, `AES-CBC`, `AES-GCM`, `signkey`), frame codecs, data models, and network state machines are independently authored in pure Rust.
* **Zero Bundled Copyrighted Media:** No proprietary artwork, official application icons, copyrighted sticker sets, audio notification files, or font binaries belonging to VNG Corporation or third parties may be distributed in this Git repository. All media must be loaded dynamically by the client at runtime under the user's authenticated session.

---

## 3. Naming & Branding Policy

To eliminate any potential for consumer confusion and fully comply with trademark protection standards:
1. **Product Name:** The application is branded exclusively as **Alolite**. Binary packages, desktop entries, and public distribution metadata must strictly use `alolite`.
2. **Internal Module Structure:** Protocol implementation layers are named with neutral technical terminology (`src/protocol/`) rather than trademarked commercial brands.
3. **Application Artwork:** All icons, branding assets, and graphical UI elements must be original works authored for Alolite or sourced from permissive open-source licenses (e.g., Lucide icons, SIL OFL fonts). Official brand logos (such as the Zalo rounded square "Z" logo) are strictly prohibited.

---

## 4. Acceptable Use Policy (AUP)

Users and contributors of Alolite agree to the following terms:
1. **Personal & Legitimate Use:** Alolite is designed solely for individual users accessing their own lawful personal accounts on non-supported platforms (such as Linux desktop environments).
2. **Prohibited Activities:**
   - **No Automated Spamming or Harassment:** Using this software for automated unsolicited bulk messaging, automated calling, or harassment is strictly prohibited.
   - **No Data Scraping:** Using this software to harvest, scrape, or extract personal information from other users without consent is strictly prohibited.
   - **No Denial of Service:** Circumventing rate limits, exploiting vulnerabilities, or overloading service infrastructure is strictly prohibited.
3. **Non-Commercial Nature:** Alolite is distributed free of charge under the Alolite Source-Available & Non-Commercial License (see `LICENSE`). It contains no advertisements, telemetry tracking, premium tiers, or commercial monetization mechanisms.

---

## 5. Disclaimer of Warranties & Limitation of Liability

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
