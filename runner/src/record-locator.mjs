import { validateLocator } from './recipe-validation.mjs';

/** Selection mode suppresses page actions; it never records values or passwords. */
export async function installLocatorPicker(page,onPick) {
  await page.exposeBinding('__ownveroPick',async(_source,locator)=>{validateLocator(locator);await onPick(locator)});
  const install=()=>{
    const pick=event=>{
      event.preventDefault();event.stopImmediatePropagation();
      if(!event.isTrusted)return;
      const target=event.target;
      if(!(target instanceof Element) || target.closest('[type="password"],[type="file"],[autocomplete*="password"],[autocomplete*="cc-"]'))return;
      const testId=target.getAttribute('data-testid');
      const label=target.getAttribute('aria-label');
      const role=target.getAttribute('role')??({BUTTON:'button',A:'link'}[target.tagName]);
      const name=label??target.textContent?.trim();
      const locator=testId?{kind:'test_id',value:testId}:role&&name?{kind:'role',role,name}:null;
      if(locator && Object.values(locator).every(value=>value.length<=120))void window.__ownveroPick(locator);
    };
    window.addEventListener('click',pick,true);
    document.addEventListener('submit',event=>{event.preventDefault();event.stopImmediatePropagation()},true);
    document.addEventListener('keydown',event=>{if(event.key==='Enter'){event.preventDefault();event.stopImmediatePropagation()}},true);
  };
  await page.addInitScript(install);await page.evaluate(install);
}
