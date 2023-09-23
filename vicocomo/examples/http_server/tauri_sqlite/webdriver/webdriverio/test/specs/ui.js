describe('Vicocomo example - Tauri w Sqlite', () => {
  it('abuse Mocha', async () => {
    const count = await $('#count');
    const extraTxt = await $('#extra-txt');
    await expect(count).toHaveValue('-4711');
    await findAndClick('#delete-btn');
    await expect(count).toHaveValue('0');
    await setValue('#count', '42');
    await findAndClick('#count-ok');
    await setValue('#count', '-42');
    await findAndClick('#cancel-lnk');
    await expect(count).toHaveValue('42');
    await expect(extraTxt).toHaveText('cancelled');
    const color = await extraTxt.getCSSProperty('color');
    await expect(color.parsed.hex).toBe('#ff0000');
  })
})

// https://github.com/tauri-apps/tauri/issues/6541
const findAndClick = async (selector) => {
  const element = await $(selector);
  await element.waitForClickable();
  await browser.execute('arguments[0].click();', element);
}

// https://github.com/tauri-apps/tauri/issues/6541
const setValue = async (selector, value) => {
  const field = await $(selector);
  await browser.execute('arguments[0].value="' + value + '"', field);
  await browser.execute(
    'arguments[0].dispatchEvent(new Event("input", { bubbles: true }))',
    field
  );
}
