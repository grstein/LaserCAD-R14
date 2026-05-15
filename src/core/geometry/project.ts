import { camera } from '@/render/camera.js';

export const project = {
  /** @param {{x:number,y:number}} screenPt @param {object} [cam] @returns {{x:number,y:number}} */
  worldFromScreen(screenPt, cam) {
    return camera.worldFromScreen(screenPt, cam);
  },

  /** @param {{x:number,y:number}} worldPt @param {object} [cam] @returns {{x:number,y:number}} */
  screenFromWorld(worldPt, cam) {
    return camera.screenFromWorld(worldPt, cam);
  },
};
