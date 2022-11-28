import { api } from "../variables"

export const checkAvailability =  () => {
    console.log("Checking availability")
    return fetch(`${api}/health`).then((res) => {
        return res.ok
    }).catch((e) => {
        console.log(e);
        return false;
    })
    
}