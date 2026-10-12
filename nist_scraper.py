import requests
from bs4 import BeautifulSoup
import csv
import time
import sys

def scrape_nist(formula):
    # Search URL for formula
    url = f"https://webbook.nist.gov/cgi/cbook.cgi?Formula={formula}&NoIon=on&Units=SI"
    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
    }
    
    try:
        response = requests.get(url, headers=headers)
        if response.status_code != 200:
            print(f"[{formula}] Error: HTTP {response.status_code}")
            return None
        
        soup = BeautifulSoup(response.text, 'html.parser')
        
        # Check if multiple species are returned
        if "Species matching formula" in soup.text:
            print(f"[{formula}] Multiple matches found. Please be more specific or extract from the search list.")
            return None
            
        # If it's a direct page, find the Thermochemistry data link
        thermo_link = None
        for a in soup.find_all('a'):
            if "Thermochemistry data" in a.text or "Gas phase thermochemistry data" in a.text or "Condensed phase thermochemistry data" in a.text:
                thermo_link = a.get('href')
                break
                
        if thermo_link:
            full_url = "https://webbook.nist.gov" + thermo_link
            time.sleep(1) # Be nice to NIST
            resp2 = requests.get(full_url, headers=headers)
            soup2 = BeautifulSoup(resp2.text, 'html.parser')
            
            # Find the specific tables
            hf_gas = None
            hf_solid = None
            hf_liquid = None
            s_gas = None
            s_solid = None
            s_liquid = None
            
            name_tag = soup2.find('h1', {'id': 'Top'})
            name = name_tag.text if name_tag else formula
            
            # We would parse the tables here to find delta f H (gas), delta f H (solid), etc.
            # For this MVP scraper, let's extract generic rows containing \Delta_f H^\circ
            for tr in soup2.find_all('tr'):
                cells = tr.find_all(['th', 'td'])
                if len(cells) >= 2:
                    text = cells[0].text
                    val = cells[1].text.strip()
                    try:
                        # Extract first number
                        import re
                        num_match = re.search(r'[-+]?\d*\.\d+|\d+', val)
                        if num_match:
                            num = float(num_match.group())
                            if "fH°gas" in text: hf_gas = num
                            elif "fH°solid" in text: hf_solid = num
                            elif "fH°liquid" in text: hf_liquid = num
                            elif "S°gas" in text: s_gas = num
                            elif "S°solid" in text: s_solid = num
                            elif "S°liquid" in text: s_liquid = num
                    except:
                        pass
            
            # Pack results
            results = []
            if hf_gas is not None and s_gas is not None:
                results.append((name + " (g)", formula, 0.0, hf_gas, s_gas, 0))
            if hf_liquid is not None and s_liquid is not None:
                results.append((name + " (l)", formula, 0.0, hf_liquid, s_liquid, 1))
            if hf_solid is not None and s_solid is not None:
                results.append((name + " (s)", formula, 0.0, hf_solid, s_solid, 2))
                
            print(f"[{formula}] Found {len(results)} phases.")
            return results
        else:
            print(f"[{formula}] No thermochemistry data found.")
            return None
            
    except Exception as e:
        print(f"[{formula}] Exception: {e}")
        return None

if __name__ == '__main__':
    print("NIST WebBook Scraper v1.0")
    try:
        with open("formulas.txt", "r") as f:
            formulas = [line.strip() for line in f if line.strip()]
    except FileNotFoundError:
        formulas = ["Fe2O3", "NaCl", "H2O", "CH4"]
        print("formulas.txt not found. Using default test formulas.")
        
    all_data = []
    
    for form in formulas:
        res = scrape_nist(form)
        if res:
            all_data.extend(res)
        time.sleep(2) # Mandatory 2s delay to prevent IP ban
        
    if all_data:
        with open("nist_database.csv", "w", newline='') as f:
            writer = csv.writer(f)
            writer.writerow(["Nombre", "Formula", "Masa_Molar", "Hf", "Sf", "Fase"])
            for row in all_data:
                writer.writerow(row)
        print(f"Successfully saved {len(all_data)} records to nist_database.csv")
    else:
        print("No data extracted.")
