import type {PageLoad} from './$types';
import {loadAppIcon, loadImage} from "../../app";
import type {WelcomeMode} from "../../bindings";

export type GenderImages = { male: string, female: string, other: string }
export type SipImages = { full: string, half: string, sip3: string, sip2: string, sip1: string }
export type ReminderImages = { woman: string, man: string }

function getMode(): WelcomeMode {
    let mode = new URLSearchParams(window.location.search).get("mode");
    if (mode === "Complete" || mode === "OnlySipSettings") {
        return mode;
    } else {
        return "Complete";
    }
}


/** @type {import('./$types').PageLoad} */
export const load: PageLoad = async (): Promise<{
    iconPath: string,
    welcomePath: string,
    welcomeMode: WelcomeMode,
    genderImages: GenderImages,
    sipImages: SipImages,
    reminderImages: ReminderImages
}> => {
    return {
        iconPath: await loadAppIcon(),
        welcomePath: await loadImage("welcome/dn-water-glass.png"),
        welcomeMode: getMode(),
        genderImages:
            {
                male: await loadImage("welcome/gender/male.png"),
                female:
                    await loadImage("welcome/gender/female.png"),
                other:
                    await loadImage("welcome/gender/other.png"),
            }
        ,
        sipImages: {
            full: await loadImage("welcome/cups/full.png"),
            half:
                await loadImage("welcome/cups/half.png"),
            sip3:
                await loadImage("welcome/cups/sip3.png"),
            sip2:
                await loadImage("welcome/cups/sip2.png"),
            sip1:
                await loadImage("welcome/cups/sip1.png"),
        }
        ,
        reminderImages: {
            man: await loadImage("welcome/reminder/man.png"),
            woman:
                await loadImage("welcome/reminder/woman.png"),
        }
    }
        ;
};